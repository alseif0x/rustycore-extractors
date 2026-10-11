// SPDX-License-Identifier: GPL-3.0-or-later
//! Offline, read-only verification of Forever v2/v3 and Rust-local v4.
//!
//! `forever-db2-verify` proves that a directory published by
//! `forever-db2-extract` is self-consistent. It never opens the client, CASC,
//! a network socket or a remote listfile: it only reads the manifest and the
//! `.db2` files already on disk and recomputes their hashes independently.
//!
//! Version dispatch accepts v2, v3 and v4 explicitly and rejects unknown versions.
//! Each schema is validated by an in-tree bounded JSON parser and an
//! explicit field/invariant validator rather than a JSON-Schema dependency,
//! because the crate has no JSON dependency and the dependency/network policy
//! forbids adding one for this pass. The validator mirrors
//! `schemas/forever-manifest-v2.schema.json` and the separate v3 catalog schema;
//! a successful process exit is not
//! format parity or reader/runtime acceptance.
//!
//! Invariant order (first failure wins): manifest present/parse/schema, run
//! completeness, path safety, inventory, table identity, row decode and the
//! declared network/privacy/scope constants.
use crate::sha256;
use forever_db2::{CharacterTable, DecodedTable, Metadata};
use std::collections::BTreeSet;
use std::fs;
use std::io::{Cursor, Read};
use std::path::Path;

#[cfg(test)]
mod tests;

/// Companion document names from `contracts/forever-contract-v2.json`.
pub const MANIFEST_NAME: &str = "forever-manifest.json";
pub const REPORT_NAME: &str = "report.json";
pub const MARKER_NAME: &str = ".rustycore-forever-incomplete.json";

/// Inspection policy limits, not native format limits.
const MAX_MANIFEST_BYTES: u64 = 4 * 1024 * 1024;
const MAX_ARTIFACT_BYTES: u64 = 16 * 1024 * 1024;
// Bound recursion before parsing any untrusted JSON document. Valid manifests
// need only a handful of levels; this is an inspection policy.
const MAX_JSON_DEPTH: u32 = 64;

fn bounded_json_depth(text: &str) -> bool {
    let mut depth = 0_u32;
    let mut in_string = false;
    let mut escaped = false;
    for byte in text.bytes() {
        if in_string {
            if escaped {
                escaped = false;
            } else if byte == b'\\' {
                escaped = true;
            } else if byte == b'"' {
                in_string = false;
            }
        } else {
            match byte {
                b'"' => in_string = true,
                b'{' | b'[' => {
                    depth += 1;
                    if depth > MAX_JSON_DEPTH {
                        return false;
                    }
                }
                b'}' | b']' => {
                    let Some(next) = depth.checked_sub(1) else {
                        return false;
                    };
                    depth = next;
                }
                _ => {}
            }
        }
    }
    depth == 0 && !in_string
}

/// Pinned compatibility oracle, matching the manifest schema and `manifest.rs`.
const ORACLE_COMMIT: &str = "02245dcd245e7433e524577656177723d3e4992e";
const SCOPE: &str = "explicit_db2_batch";

/// Failure codes and their contract exit bindings.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Code {
    Verified,
    InvalidInvocation,
    UnsupportedClient,
    InputIo,
    IncompleteRun,
    UnsafePath,
    InventoryMismatch,
    TableIdentityMismatch,
    RowDecodeMismatch,
    ManifestSchemaInvalid,
    Internal,
}

impl Code {
    /// Stable code string; the uppercase codes are the contract `failure_codes`.
    pub fn name(self) -> &'static str {
        match self {
            Self::Verified => "verified",
            Self::InvalidInvocation => "invalid_invocation",
            Self::UnsupportedClient => "unsupported_client",
            Self::InputIo => "input_io",
            Self::IncompleteRun => "INCOMPLETE_RUN",
            Self::UnsafePath => "UNSAFE_PATH",
            Self::InventoryMismatch => "INVENTORY_MISMATCH",
            Self::TableIdentityMismatch => "TABLE_IDENTITY_MISMATCH",
            Self::RowDecodeMismatch => "ROW_DECODE_MISMATCH",
            Self::ManifestSchemaInvalid => "MANIFEST_SCHEMA_INVALID",
            Self::Internal => "internal",
        }
    }

    /// Process exit code from the v2 contract verification table.
    pub fn exit(self) -> u8 {
        match self {
            Self::Verified => 0,
            Self::InvalidInvocation | Self::ManifestSchemaInvalid => 2,
            Self::UnsupportedClient => 3,
            Self::InputIo => 4,
            Self::IncompleteRun => 8,
            Self::UnsafePath | Self::InventoryMismatch => 9,
            Self::TableIdentityMismatch | Self::RowDecodeMismatch => 9,
            Self::Internal => 10,
        }
    }

    fn status(self) -> &'static str {
        match self {
            Self::Verified => "verified",
            _ => "failed",
        }
    }
}

/// Stable, path-free verification result.
pub struct VerifyReport {
    code: Code,
    build: Option<u32>,
    artifacts_verified: usize,
}

impl VerifyReport {
    /// Build a failure report for a code detected before any manifest was read.
    pub fn failure(code: Code) -> Self {
        Self {
            code,
            build: None,
            artifacts_verified: 0,
        }
    }

    /// Process exit code for this result.
    pub fn exit_code(&self) -> u8 {
        self.code.exit()
    }

    /// Single-line stable JSON report. Never contains a local path.
    pub fn json(&self) -> String {
        format!(
            "{{\"report_version\":1,\"scope\":\"forever_output_verification\",\
            \"status\":\"{}\",\"code\":\"{}\",\"exit_code\":{},\"offline\":true,\
            \"read_only\":true,\"network\":\"local\",\"full_client_extraction\":false,\
            \"paths_redacted\":true,\"build\":{},\"artifacts_verified\":{}}}",
            self.code.status(),
            self.code.name(),
            self.code.exit(),
            self.build
                .map_or_else(|| "null".to_owned(), |build| build.to_string()),
            self.artifacts_verified
        )
    }
}

/// Verify an existing output directory. Never opens the client or CASC.
pub fn verify(output: &Path) -> VerifyReport {
    match run(output) {
        Ok(report) => report,
        Err(code) => VerifyReport::failure(code),
    }
}

fn run(output: &Path) -> Result<VerifyReport, Code> {
    // The output directory must be a real directory, never a symlink.
    let metadata = fs::symlink_metadata(output).map_err(|_| Code::InputIo)?;
    if metadata.file_type().is_symlink() {
        return Err(Code::UnsafePath);
    }
    if !metadata.is_dir() {
        return Err(Code::InputIo);
    }

    // 1. Manifest must exist, parse and validate against its explicit version.
    let manifest_path = output.join(MANIFEST_NAME);
    let manifest_meta = fs::symlink_metadata(&manifest_path).map_err(|_| Code::InputIo)?;
    if manifest_meta.file_type().is_symlink() {
        return Err(Code::UnsafePath);
    }
    if !manifest_meta.is_file() {
        return Err(Code::InputIo);
    }
    let bytes = read_capped(&manifest_path, MAX_MANIFEST_BYTES).map_err(|_| Code::InputIo)?;
    let text = std::str::from_utf8(&bytes).map_err(|_| Code::ManifestSchemaInvalid)?;
    let document = json::parse(text).map_err(|_| Code::ManifestSchemaInvalid)?;
    let manifest = validate(&document).map_err(|_| Code::ManifestSchemaInvalid)?;
    if !crate::SUPPORTED_CLIENTS
        .iter()
        .any(|(_, build)| *build == manifest.build)
    {
        return Err(Code::UnsupportedClient);
    }

    // 2. No partial file, no incomplete marker and a complete run.
    let marker_present = output.join(MARKER_NAME).symlink_metadata().is_ok();
    if manifest.status != Status::Complete
        || manifest.incomplete_marker_present
        || marker_present
        || has_part_file(output)
    {
        return Err(Code::IncompleteRun);
    }

    // 3. Every published path is a simple name and never a symlink.
    let complete: Vec<&Artifact> = manifest
        .artifacts
        .iter()
        .filter(|artifact| artifact.complete)
        .collect();
    for artifact in &complete {
        if !valid_relative_path(&artifact.relative_path) {
            return Err(Code::UnsafePath);
        }
        let path = output.join(&artifact.relative_path);
        if let Ok(link) = fs::symlink_metadata(&path)
            && link.file_type().is_symlink()
        {
            return Err(Code::UnsafePath);
        }
    }

    // 4-6. Recompute hashes, reopen metadata and re-decode claimed bodies.
    let loaded = load_complete(
        output,
        &complete,
        manifest.schema_catalog || manifest.acquisition.is_some(),
        manifest.max_file_bytes,
    )
    .map_err(|_| Code::InventoryMismatch)?;
    check_integrity(output, &complete, &loaded, &manifest.integrity)
        .map_err(|_| Code::InventoryMismatch)?;
    check_tables(
        &complete,
        &loaded,
        &manifest.tables,
        manifest.schema_catalog,
    )
    .map_err(|_| Code::TableIdentityMismatch)?;
    check_rows(&complete, &loaded, manifest.schema_catalog).map_err(|_| Code::RowDecodeMismatch)?;

    // 7. Declared constants; network/privacy/scope are schema consts, and the
    // companion report is additionally checked for `full_client_extraction`.
    check_declared_constants(output, &manifest).map_err(|_| Code::ManifestSchemaInvalid)?;

    Ok(VerifyReport {
        code: Code::Verified,
        build: Some(manifest.build),
        artifacts_verified: complete.len(),
    })
}

/// Read at most `max` bytes; larger files are an explicit failure.
fn read_capped(path: &Path, max: u64) -> Result<Vec<u8>, Invalid> {
    let file = fs::File::open(path).map_err(|_| Invalid)?;
    let mut bytes = Vec::new();
    file.take(max + 1)
        .read_to_end(&mut bytes)
        .map_err(|_| Invalid)?;
    if bytes.len() as u64 > max {
        return Err(Invalid);
    }
    Ok(bytes)
}

fn has_part_file(output: &Path) -> bool {
    fs::read_dir(output).is_ok_and(|entries| {
        entries
            .filter_map(Result::ok)
            .any(|entry| entry.file_name().to_string_lossy().ends_with(".part"))
    })
}

fn db2_files(output: &Path) -> Result<Vec<String>, Invalid> {
    let entries = fs::read_dir(output).map_err(|_| Invalid)?;
    let mut names = Vec::new();
    for entry in entries {
        let entry = entry.map_err(|_| Invalid)?;
        let name = entry.file_name().to_string_lossy().into_owned();
        if name.ends_with(".db2") {
            names.push(name);
        }
    }
    names.sort();
    Ok(names)
}

fn load_complete(
    output: &Path,
    complete: &[&Artifact],
    bounded_batch: bool,
    max_file_bytes: u64,
) -> Result<Vec<Vec<u8>>, Invalid> {
    let mut loaded = Vec::with_capacity(complete.len());
    let mut total = 0u64;
    for artifact in complete {
        let budget = if bounded_batch {
            crate::batch::MAX_BATCH_BYTES
                .checked_sub(total)
                .ok_or(Invalid)?
                .min(max_file_bytes)
        } else {
            max_file_bytes
        };
        let bytes = read_capped(&output.join(&artifact.relative_path), budget)?;
        total = total.checked_add(bytes.len() as u64).ok_or(Invalid)?;
        loaded.push(bytes);
    }
    Ok(loaded)
}

fn check_integrity(
    output: &Path,
    complete: &[&Artifact],
    loaded: &[Vec<u8>],
    integrity: &Integrity,
) -> Result<(), Invalid> {
    let mut total = 0u64;
    let mut lines = Vec::with_capacity(complete.len());
    for (artifact, bytes) in complete.iter().zip(loaded) {
        if bytes.len() as u64 != artifact.bytes {
            return Err(Invalid);
        }
        if sha256::hex(bytes) != artifact.sha256 {
            return Err(Invalid);
        }
        total = total.checked_add(artifact.bytes).ok_or(Invalid)?;
        let sha = &artifact.sha256;
        let name = &artifact.name;
        lines.push(format!("{sha}  {name}.db2"));
    }
    if integrity.file_count != complete.len() as u64 {
        return Err(Invalid);
    }
    if integrity.total_bytes != total {
        return Err(Invalid);
    }
    // Same inventory text as `manifest.rs`: whole-line UTF-8 byte order.
    lines.sort_by(|left, right| left.as_bytes().cmp(right.as_bytes()));
    let inventory = lines.join("\n");
    if sha256::hex(inventory.as_bytes()) != integrity.inventory_sha256 {
        return Err(Invalid);
    }
    let listed: BTreeSet<&str> = complete
        .iter()
        .map(|artifact| artifact.relative_path.as_str())
        .collect();
    for name in db2_files(output)? {
        if !listed.contains(name.as_str()) {
            return Err(Invalid);
        }
    }
    Ok(())
}

fn check_tables(
    complete: &[&Artifact],
    loaded: &[Vec<u8>],
    tables: &[Table],
    schema_catalog: bool,
) -> Result<(), Invalid> {
    for (artifact, bytes) in complete.iter().zip(loaded) {
        let metadata = Metadata::read(&mut Cursor::new(bytes)).map_err(|_| Invalid)?;
        let header = metadata.header;
        let table = tables
            .iter()
            .find(|table| table.name == artifact.name)
            .ok_or(Invalid)?;
        if table.file_id != artifact.file_id
            || u64::from(header.table_hash) != table.table_hash
            || u64::from(header.layout_hash) != table.layout_hash
            || u64::from(header.field_count) != table.field_count
            || header.total_field_count != header.field_count
        {
            return Err(Invalid);
        }
        if let Some(known) = resolve_table(&artifact.name, schema_catalog)?
            && !known.matches(&header)
        {
            return Err(Invalid);
        }
        if schema_catalog {
            let known = resolve_table(&artifact.name, true)?.ok_or(Invalid)?;
            if artifact.file_id != u64::from(known.file_id())
                || table.meta_index_field != i64::from(known.schema_identity().2)
                || artifact.declared_records != u64::from(header.record_count)
            {
                return Err(Invalid);
            }
        }
    }
    Ok(())
}

fn resolve_table(name: &str, schema_catalog: bool) -> Result<Option<CharacterTable>, Invalid> {
    if schema_catalog {
        CharacterTable::from_catalog(name)
            .map_err(|_| Invalid)?
            .map(Some)
            .ok_or(Invalid)
    } else {
        Ok(CharacterTable::named(name))
    }
}

fn check_rows(
    complete: &[&Artifact],
    loaded: &[Vec<u8>],
    schema_catalog: bool,
) -> Result<(), Invalid> {
    for (artifact, bytes) in complete.iter().zip(loaded) {
        if !artifact.record_data_decoded {
            continue;
        }
        let table = resolve_table(&artifact.name, schema_catalog)?.ok_or(Invalid)?;
        let decoded = DecodedTable::read(bytes, table).map_err(|_| Invalid)?;
        if artifact.decoded_rows != Some(decoded.rows().len() as u64) {
            return Err(Invalid);
        }
    }
    Ok(())
}

fn check_declared_constants(output: &Path, manifest: &Manifest) -> Result<(), Invalid> {
    let schema_catalog = manifest.schema_catalog;
    let acquisition = manifest.acquisition;
    // `network`, `privacy` and `scope` are schema consts already enforced by
    // `validate`. `full_client_extraction` lives only in the companion
    // `report.json`; when that file is present it must still declare `false`.
    let path = output.join(REPORT_NAME);
    if acquisition.is_some()
        && fs::symlink_metadata(&path)
            .map_err(|_| Invalid)?
            .file_type()
            .is_symlink()
    {
        return Err(Invalid);
    }
    if !fs::metadata(&path)
        .map(|meta| meta.is_file())
        .unwrap_or(false)
    {
        return if acquisition.is_some() {
            Err(Invalid)
        } else {
            Ok(())
        };
    }
    let bytes = read_capped(&path, MAX_MANIFEST_BYTES)?;
    let text = std::str::from_utf8(&bytes).map_err(|_| Invalid)?;
    let report = json::parse(text).map_err(|_| Invalid)?;
    if let Some(profile) = acquisition {
        const_u64(report.get("report_version").ok_or(Invalid)?, 2)?;
        const_u64(
            report.get("build").ok_or(Invalid)?,
            u64::from(manifest.build),
        )?;
        const_str(report.get("locale").ok_or(Invalid)?, &manifest.locale)?;
        const_str(report.get("status").ok_or(Invalid)?, "success")?;
        for key in ["offline", "paths_redacted", "all_requested_files_complete"] {
            if report.get(key).and_then(json::Json::as_bool) != Some(true) {
                return Err(Invalid);
            }
        }
        let observed = crate::acquisition::Profile::decode(
            report.get("acquisition").ok_or(Invalid)?,
            crate::batch::MAX_FILE_BYTES,
        )
        .map_err(|_| Invalid)?;
        if observed != profile {
            return Err(Invalid);
        }
        const_str(
            report.get("schema_selection").ok_or(Invalid)?,
            if schema_catalog {
                "catalog"
            } else {
                "reviewed"
            },
        )?;
        const_str(
            report.get("scope").ok_or(Invalid)?,
            if schema_catalog {
                crate::manifest::CATALOG_SCOPE
            } else {
                SCOPE
            },
        )?;
        if !schema_catalog && report.get("schema_catalog").is_some() {
            return Err(Invalid);
        }
    }
    if schema_catalog {
        const_str(
            report.get("scope").ok_or(Invalid)?,
            crate::manifest::CATALOG_SCOPE,
        )?;
        const_str(
            report.get("schema_catalog").ok_or(Invalid)?,
            forever_db2::catalog::CATALOG_ID,
        )?;
    }
    match report.get("full_client_extraction") {
        Some(value) if value.as_bool() == Some(false) => Ok(()),
        Some(_) => Err(Invalid),
        None if schema_catalog || acquisition.is_some() => Err(Invalid),
        None => Ok(()),
    }
}

// ---------------------------------------------------------------------------
// Bounded JSON parser (objects, arrays, strings, integers, booleans, null).
// ---------------------------------------------------------------------------
pub(crate) mod json {
    /// Parsed JSON value. Integers only, matching the v2 manifest's types.
    #[derive(Debug, Clone, PartialEq)]
    pub(crate) enum Json {
        Null,
        Bool(bool),
        Number(i64),
        String(String),
        Array(Vec<Json>),
        Object(Vec<(String, Json)>),
    }

    impl Json {
        pub(super) fn get(&self, key: &str) -> Option<&Json> {
            match self {
                Self::Object(members) => members
                    .iter()
                    .find(|(name, _)| name.as_str() == key)
                    .map(|(_, value)| value),
                _ => None,
            }
        }

        pub(super) fn as_bool(&self) -> Option<bool> {
            match self {
                Self::Bool(value) => Some(*value),
                _ => None,
            }
        }
    }

    pub(crate) fn parse(text: &str) -> Result<Json, ()> {
        if !super::bounded_json_depth(text) {
            return Err(());
        }
        let mut parser = Parser {
            bytes: text.as_bytes(),
            at: 0,
        };
        parser.whitespace();
        let value = parser.value()?;
        parser.whitespace();
        if parser.at == parser.bytes.len() {
            Ok(value)
        } else {
            Err(())
        }
    }

    struct Parser<'a> {
        bytes: &'a [u8],
        at: usize,
    }

    impl Parser<'_> {
        fn peek(&self) -> Option<u8> {
            self.bytes.get(self.at).copied()
        }

        fn whitespace(&mut self) {
            while matches!(self.peek(), Some(b' ' | b'\n' | b'\r' | b'\t')) {
                self.at += 1;
            }
        }

        fn value(&mut self) -> Result<Json, ()> {
            match self.peek().ok_or(())? {
                b'{' => self.object(),
                b'[' => self.array(),
                b'"' => Ok(Json::String(self.string()?)),
                b't' => {
                    self.literal(b"true")?;
                    Ok(Json::Bool(true))
                }
                b'f' => {
                    self.literal(b"false")?;
                    Ok(Json::Bool(false))
                }
                b'n' => {
                    self.literal(b"null")?;
                    Ok(Json::Null)
                }
                b'-' | b'0'..=b'9' => self.number(),
                _ => Err(()),
            }
        }

        fn literal(&mut self, word: &[u8]) -> Result<(), ()> {
            if self.bytes[self.at..].starts_with(word) {
                self.at += word.len();
                Ok(())
            } else {
                Err(())
            }
        }

        fn object(&mut self) -> Result<Json, ()> {
            self.at += 1;
            let mut members: Vec<(String, Json)> = Vec::new();
            self.whitespace();
            if self.peek() == Some(b'}') {
                self.at += 1;
                return Ok(Json::Object(members));
            }
            loop {
                self.whitespace();
                if self.peek() != Some(b'"') {
                    return Err(());
                }
                let key = self.string()?;
                if members.iter().any(|(name, _)| name == &key) {
                    return Err(());
                }
                self.whitespace();
                if self.peek() != Some(b':') {
                    return Err(());
                }
                self.at += 1;
                self.whitespace();
                let value = self.value()?;
                members.push((key, value));
                self.whitespace();
                match self.peek() {
                    Some(b',') => self.at += 1,
                    Some(b'}') => {
                        self.at += 1;
                        return Ok(Json::Object(members));
                    }
                    _ => return Err(()),
                }
            }
        }

        fn array(&mut self) -> Result<Json, ()> {
            self.at += 1;
            let mut values = Vec::new();
            self.whitespace();
            if self.peek() == Some(b']') {
                self.at += 1;
                return Ok(Json::Array(values));
            }
            loop {
                self.whitespace();
                values.push(self.value()?);
                self.whitespace();
                match self.peek() {
                    Some(b',') => self.at += 1,
                    Some(b']') => {
                        self.at += 1;
                        return Ok(Json::Array(values));
                    }
                    _ => return Err(()),
                }
            }
        }

        fn number(&mut self) -> Result<Json, ()> {
            let start = self.at;
            if self.peek() == Some(b'-') {
                self.at += 1;
            }
            let digits_start = self.at;
            while matches!(self.peek(), Some(b'0'..=b'9')) {
                self.at += 1;
            }
            if self.at == digits_start {
                return Err(());
            }
            let digits = &self.bytes[digits_start..self.at];
            if digits.len() > 1 && digits[0] == b'0' {
                return Err(());
            }
            if matches!(self.peek(), Some(b'.' | b'e' | b'E')) {
                return Err(());
            }
            let text = std::str::from_utf8(&self.bytes[start..self.at]).map_err(|_| ())?;
            text.parse::<i64>().map(Json::Number).map_err(|_| ())
        }

        fn string(&mut self) -> Result<String, ()> {
            self.at += 1;
            let mut out = String::new();
            loop {
                let byte = *self.bytes.get(self.at).ok_or(())?;
                match byte {
                    b'"' => {
                        self.at += 1;
                        return Ok(out);
                    }
                    b'\\' => {
                        self.at += 1;
                        let escape = *self.bytes.get(self.at).ok_or(())?;
                        self.at += 1;
                        match escape {
                            b'"' => out.push('"'),
                            b'\\' => out.push('\\'),
                            b'/' => out.push('/'),
                            b'b' => out.push('\u{0008}'),
                            b'f' => out.push('\u{000c}'),
                            b'n' => out.push('\n'),
                            b'r' => out.push('\r'),
                            b't' => out.push('\t'),
                            b'u' => {
                                let code = self.hex4()?;
                                if (0xD800..=0xDBFF).contains(&code) {
                                    if self.bytes.get(self.at) != Some(&b'\\')
                                        || self.bytes.get(self.at + 1) != Some(&b'u')
                                    {
                                        return Err(());
                                    }
                                    self.at += 2;
                                    let low = self.hex4()?;
                                    if !(0xDC00..=0xDFFF).contains(&low) {
                                        return Err(());
                                    }
                                    let scalar = 0x10000 + ((code - 0xD800) << 10) + (low - 0xDC00);
                                    out.push(char::from_u32(scalar).ok_or(())?);
                                } else {
                                    out.push(char::from_u32(code).ok_or(())?);
                                }
                            }
                            _ => return Err(()),
                        }
                    }
                    _ if byte < 0x20 => return Err(()),
                    _ => {
                        let text = std::str::from_utf8(&self.bytes[self.at..]).map_err(|_| ())?;
                        let character = text.chars().next().ok_or(())?;
                        out.push(character);
                        self.at += character.len_utf8();
                    }
                }
            }
        }

        fn hex4(&mut self) -> Result<u32, ()> {
            let end = self.at.checked_add(4).ok_or(())?;
            let slice = self.bytes.get(self.at..end).ok_or(())?;
            let text = std::str::from_utf8(slice).map_err(|_| ())?;
            let value = u32::from_str_radix(text, 16).map_err(|_| ())?;
            self.at = end;
            Ok(value)
        }
    }
}

// ---------------------------------------------------------------------------
// v2 manifest validator (mirrors `schemas/forever-manifest-v2.schema.json`).
// ---------------------------------------------------------------------------

/// Private validation failure; the public boundary maps it to a code.
#[derive(Debug, Clone, Copy)]
struct Invalid;

type VResult<T> = Result<T, Invalid>;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Status {
    Complete,
    Incomplete,
    Failed,
}

struct Artifact {
    name: String,
    file_id: u64,
    relative_path: String,
    complete: bool,
    bytes: u64,
    sha256: String,
    record_data_decoded: bool,
    decoded_rows: Option<u64>,
    declared_records: u64,
    schema_validated: bool,
}

struct Table {
    name: String,
    file_id: u64,
    table_hash: u64,
    layout_hash: u64,
    field_count: u64,
    meta_index_field: i64,
}

struct Integrity {
    file_count: u64,
    total_bytes: u64,
    inventory_sha256: String,
}

struct Manifest {
    status: Status,
    build: u32,
    locale: String,
    artifacts: Vec<Artifact>,
    tables: Vec<Table>,
    integrity: Integrity,
    incomplete_marker_present: bool,
    schema_catalog: bool,
    acquisition: Option<crate::acquisition::Profile>,
    max_file_bytes: u64,
}

const TOP_REQUIRED: &[&str] = &[
    "manifest_version",
    "status",
    "compatibility",
    "client",
    "extractor",
    "scope",
    "formats",
    "options",
    "network",
    "outputs",
    "timestamps",
    "privacy",
];
const TOP_OPTIONAL: &[&str] = &["run_id", "reason"];

fn validate(root: &json::Json) -> VResult<Manifest> {
    // Version dispatch precedes either contract's member validation. A v3
    // document is never interpreted as a strengthened v2 claim, and future
    // versions remain explicit failures.
    let acquired = matches!(root.get("manifest_version"), Some(json::Json::Number(4)));
    let schema_catalog = match root.get("manifest_version") {
        Some(json::Json::Number(2)) => false,
        Some(json::Json::Number(3)) => true,
        Some(json::Json::Number(4)) => match root
            .get("schema_selection")
            .and_then(|value| as_str(value).ok())
        {
            Some("reviewed") => false,
            Some("catalog") => true,
            _ => return Err(Invalid),
        },
        _ => return Err(Invalid),
    };
    let mut required = TOP_REQUIRED.to_vec();
    if acquired {
        required.extend(["acquisition", "schema_selection"]);
    }
    if schema_catalog {
        required.push("schema_catalog");
    }
    let top = members(root, &required, TOP_OPTIONAL)?;
    let acquisition = if acquired {
        Some(
            crate::acquisition::Profile::decode(
                need(top, "acquisition")?,
                crate::batch::MAX_FILE_BYTES,
            )
            .map_err(|_| Invalid)?,
        )
    } else {
        None
    };
    let max_file_bytes = acquisition.map_or(MAX_ARTIFACT_BYTES, |profile| {
        u64::from(profile.max_file_bytes())
    });
    if schema_catalog {
        validate_catalog(need(top, "schema_catalog")?)?;
    }
    if let Some(run_id) = get(top, "run_id")
        && !valid_uuid(as_str(run_id)?)
    {
        return Err(Invalid);
    }
    let status = parse_status(need(top, "status")?)?;
    if let Some(reason) = get(top, "reason") {
        nonempty_str(reason)?;
    }

    let compatibility = members(
        need(top, "compatibility")?,
        &["support", "target_build"],
        &["reason"],
    )?;
    let support = one_of_str(
        need(compatibility, "support")?,
        &["admitted", "experimental"],
    )?;
    let target = as_u64(need(compatibility, "target_build")?)?;
    if target != 70170 && target != 70205 {
        return Err(Invalid);
    }
    if let Some(reason) = get(compatibility, "reason") {
        nonempty_str(reason)?;
    }
    if support == "experimental" && get(compatibility, "reason").is_none() {
        return Err(Invalid);
    }
    if support == "admitted" && get(compatibility, "reason").is_some() {
        return Err(Invalid);
    }

    let client = members(
        need(top, "client")?,
        &["product", "version", "build", "locale"],
        &["install_fingerprint_sha256"],
    )?;
    const_str(need(client, "product")?, "wow_classic_beta")?;
    let build = as_u64(need(client, "build")?)?;
    if build != 70170 && build != 70205 {
        return Err(Invalid);
    }
    let expected_version = format!("1.60.1.{build}");
    const_str(need(client, "version")?, &expected_version)?;
    one_of_str(need(client, "locale")?, &["enUS", "esES"])?;
    if let Some(fingerprint) = get(client, "install_fingerprint_sha256")
        && !valid_hex(as_str(fingerprint)?, 64)
    {
        return Err(Invalid);
    }

    let extractor = members(
        need(top, "extractor")?,
        &[
            "name",
            "version",
            "implementation",
            "compatibility_oracle_commit",
        ],
        &["commit"],
    )?;
    const_str(need(extractor, "name")?, "forever-db2-extract")?;
    if !valid_semver(as_str(need(extractor, "version")?)?) {
        return Err(Invalid);
    }
    if let Some(commit) = get(extractor, "commit")
        && !valid_hex(as_str(commit)?, 40)
    {
        return Err(Invalid);
    }
    const_str(need(extractor, "implementation")?, "rust")?;
    const_str(
        need(extractor, "compatibility_oracle_commit")?,
        ORACLE_COMMIT,
    )?;
    const_str(
        need(top, "scope")?,
        if schema_catalog {
            crate::manifest::CATALOG_SCOPE
        } else {
            SCOPE
        },
    )?;

    let formats = members(need(top, "formats")?, &["db2", "tables"], &[])?;
    let db2 = members(need(formats, "db2")?, &["magic", "version"], &[])?;
    const_str(need(db2, "magic")?, "WDC5")?;
    const_u64(need(db2, "version")?, 5)?;
    let tables_json = need(formats, "tables")?;
    let json::Json::Array(table_items) = tables_json else {
        return Err(Invalid);
    };
    if table_items.is_empty()
        || ((schema_catalog || acquired) && table_items.len() > crate::batch::MAX_FILES)
    {
        return Err(Invalid);
    }
    let mut tables = Vec::with_capacity(table_items.len());
    for item in table_items {
        let entry = members(
            item,
            &[
                "name",
                "file_id",
                "table_hash",
                "layout_hash",
                "meta_index_field",
                "field_count",
            ],
            &[],
        )?;
        let name = as_str(need(entry, "name")?)?;
        if !valid_simple_name(name) {
            return Err(Invalid);
        }
        let file_id = as_u64(need(entry, "file_id")?)?;
        let field_count = as_u64(need(entry, "field_count")?)?;
        if file_id == 0 || field_count == 0 {
            return Err(Invalid);
        }
        // `meta_index_field` is any integer; its authority is the reviewed
        // metadata, deliberately not the native header `IndexField` word.
        let meta_index_field = as_i64(need(entry, "meta_index_field")?)?;
        tables.push(Table {
            name: name.to_owned(),
            file_id,
            table_hash: as_u64(need(entry, "table_hash")?)?,
            layout_hash: as_u64(need(entry, "layout_hash")?)?,
            field_count,
            meta_index_field,
        });
    }

    let mut required_options = vec![
        "locale",
        "table_selection",
        "manifest_sha256",
        "entry_count",
        "max_file_bytes",
        "max_batch_bytes",
    ];
    if schema_catalog {
        required_options.push("schema_catalog");
    }
    let options = members(need(top, "options")?, &required_options, &[])?;
    one_of_str(need(options, "locale")?, &["enUS", "esES"])?;
    let selection = one_of_str(
        need(options, "table_selection")?,
        &["default_character_tables", "explicit_manifest"],
    )?;
    if schema_catalog {
        const_str(
            need(options, "schema_catalog")?,
            forever_db2::catalog::CATALOG_ID,
        )?;
        if selection != "explicit_manifest" {
            return Err(Invalid);
        }
    }
    if (schema_catalog || acquired)
        && (as_str(need(options, "locale")?)? != as_str(need(client, "locale")?)?
            || target != build)
    {
        return Err(Invalid);
    }
    match need(options, "manifest_sha256")? {
        json::Json::Null if selection == "default_character_tables" => {}
        json::Json::String(hash) if selection == "explicit_manifest" && valid_hex(hash, 64) => {}
        _ => return Err(Invalid),
    }
    if !(1..=256).contains(&as_u64(need(options, "entry_count")?)?) {
        return Err(Invalid);
    }
    const_u64(need(options, "max_file_bytes")?, max_file_bytes)?;
    const_u64(need(options, "max_batch_bytes")?, 268_435_456)?;

    let network = members(
        need(top, "network")?,
        &[
            "mode",
            "allow_network",
            "remote_casc",
            "lock_sha256",
            "downloaded_bytes",
        ],
        &[],
    )?;
    const_str(need(network, "mode")?, "local")?;
    if as_bool(need(network, "allow_network")?)? {
        return Err(Invalid);
    }
    if as_bool(need(network, "remote_casc")?)? {
        return Err(Invalid);
    }
    if !matches!(need(network, "lock_sha256")?, json::Json::Null) {
        return Err(Invalid);
    }
    const_u64(need(network, "downloaded_bytes")?, 0)?;

    let timestamps = members(
        need(top, "timestamps")?,
        &["started_at", "completed_at"],
        &[],
    )?;
    if !valid_datetime(as_str(need(timestamps, "started_at")?)?) {
        return Err(Invalid);
    }
    let completed_at_is_string = match need(timestamps, "completed_at")? {
        json::Json::Null => false,
        json::Json::String(value) if valid_datetime(value) => true,
        _ => return Err(Invalid),
    };

    let privacy = members(
        need(top, "privacy")?,
        &["paths_redacted", "client_data_embedded"],
        &[],
    )?;
    if !as_bool(need(privacy, "paths_redacted")?)? {
        return Err(Invalid);
    }
    if as_bool(need(privacy, "client_data_embedded")?)? {
        return Err(Invalid);
    }

    let outputs = members(
        need(top, "outputs")?,
        &[
            "root",
            "incomplete_marker_present",
            "artifacts",
            "integrity",
        ],
        &[],
    )?;
    const_str(need(outputs, "root")?, ".")?;
    let marker = as_bool(need(outputs, "incomplete_marker_present")?)?;
    let artifacts_json = need(outputs, "artifacts")?;
    let json::Json::Array(artifact_items) = artifacts_json else {
        return Err(Invalid);
    };
    if artifact_items.is_empty()
        || ((schema_catalog || acquired) && artifact_items.len() > crate::batch::MAX_FILES)
    {
        return Err(Invalid);
    }
    let mut artifacts = Vec::with_capacity(artifact_items.len());
    for item in artifact_items {
        artifacts.push(parse_artifact(item)?);
    }
    let integrity = parse_integrity(need(outputs, "integrity")?)?;
    if schema_catalog || acquired {
        let entry_count = as_u64(need(options, "entry_count")?)? as usize;
        if tables.len() != entry_count
            || artifacts.len() != entry_count
            || integrity.total_bytes > crate::batch::MAX_BATCH_BYTES
        {
            return Err(Invalid);
        }
        let mut names = BTreeSet::new();
        let mut ids = BTreeSet::new();
        for table in &tables {
            let known = resolve_table(&table.name, schema_catalog)?;
            if known.is_some_and(|known| table.file_id != u64::from(known.file_id()))
                || table.file_id == 0
                || table.file_id >= u64::from(u32::MAX)
                || !names.insert(table.name.to_ascii_lowercase())
                || !ids.insert(table.file_id)
            {
                return Err(Invalid);
            }
        }
        let mut seen = BTreeSet::new();
        for artifact in &artifacts {
            if !seen.insert(&artifact.name)
                || !tables
                    .iter()
                    .any(|table| table.name == artifact.name && table.file_id == artifact.file_id)
                || artifact.relative_path != format!("{}.db2", artifact.name)
                || artifact.bytes > max_file_bytes
                || (schema_catalog
                    && artifact.complete
                    && (!artifact.schema_validated
                        || !artifact.record_data_decoded
                        || artifact.decoded_rows.is_none()))
                || (!artifact.complete
                    && (artifact.record_data_decoded || artifact.decoded_rows.is_some()))
            {
                return Err(Invalid);
            }
        }
    }

    match status {
        Status::Complete => {
            if marker || !completed_at_is_string {
                return Err(Invalid);
            }
            // Complete rows only: the `else` schema branch does not constrain
            // artifact status, but the complete branch does.
            if artifacts.iter().any(|artifact| !artifact.complete) {
                return Err(Invalid);
            }
        }
        Status::Incomplete | Status::Failed => {
            if !marker || get(top, "reason").is_none() {
                return Err(Invalid);
            }
        }
    }

    Ok(Manifest {
        status,
        build: build as u32,
        locale: as_str(need(client, "locale")?)?.to_owned(),
        artifacts,
        tables,
        integrity,
        incomplete_marker_present: marker,
        schema_catalog,
        acquisition,
        max_file_bytes,
    })
}

fn validate_catalog(value: &json::Json) -> VResult<()> {
    let facts = members(
        value,
        &[
            "id",
            "source_commit",
            "source_blob",
            "source_sha256",
            "facts_sha256",
        ],
        &[],
    )?;
    for (key, expected) in [
        ("id", forever_db2::catalog::CATALOG_ID),
        ("source_commit", forever_db2::catalog::SOURCE_COMMIT),
        ("source_blob", forever_db2::catalog::SOURCE_GIT_BLOB),
        ("source_sha256", forever_db2::catalog::SOURCE_SHA256),
        ("facts_sha256", crate::manifest::CATALOG_FACTS_SHA256),
    ] {
        const_str(need(facts, key)?, expected)?;
    }
    Ok(())
}

fn parse_artifact(value: &json::Json) -> VResult<Artifact> {
    let entry = members(
        value,
        &[
            "name",
            "file_id",
            "relative_path",
            "status",
            "bytes",
            "sha256",
            "declared_records",
            "table_schema_validated",
            "record_data_decoded",
            "decoded_rows",
        ],
        &["code"],
    )?;
    let name = as_str(need(entry, "name")?)?;
    if !valid_simple_name(name) {
        return Err(Invalid);
    }
    let file_id = as_u64(need(entry, "file_id")?)?;
    if file_id == 0 {
        return Err(Invalid);
    }
    let relative_path = as_str(need(entry, "relative_path")?)?;
    if !valid_relative_path(relative_path) {
        return Err(Invalid);
    }
    let complete = one_of_str(need(entry, "status")?, &["complete", "error"])? == "complete";
    match (complete, get(entry, "code")) {
        (true, Some(_)) => return Err(Invalid),
        (false, None) => return Err(Invalid),
        (false, Some(code)) if !valid_lower_code(as_str(code)?) => return Err(Invalid),
        _ => {}
    }
    let bytes = as_u64(need(entry, "bytes")?)?;
    let sha256 = as_str(need(entry, "sha256")?)?;
    if !valid_hex(sha256, 64) {
        return Err(Invalid);
    }
    let declared_records = as_u64(need(entry, "declared_records")?)?;
    let schema_validated = as_bool(need(entry, "table_schema_validated")?)?;
    let record_data_decoded = as_bool(need(entry, "record_data_decoded")?)?;
    let decoded_rows = match need(entry, "decoded_rows")? {
        json::Json::Null => None,
        json::Json::Number(value) if *value >= 0 => Some(*value as u64),
        _ => return Err(Invalid),
    };
    if record_data_decoded && decoded_rows.is_none() {
        return Err(Invalid);
    }
    Ok(Artifact {
        name: name.to_owned(),
        file_id,
        relative_path: relative_path.to_owned(),
        complete,
        bytes,
        sha256: sha256.to_owned(),
        record_data_decoded,
        decoded_rows,
        declared_records,
        schema_validated,
    })
}

fn parse_integrity(value: &json::Json) -> VResult<Integrity> {
    let entry = members(
        value,
        &["algorithm", "file_count", "total_bytes", "inventory_sha256"],
        &[],
    )?;
    const_str(need(entry, "algorithm")?, "sha256")?;
    let inventory_sha256 = as_str(need(entry, "inventory_sha256")?)?;
    if !valid_hex(inventory_sha256, 64) {
        return Err(Invalid);
    }
    Ok(Integrity {
        file_count: as_u64(need(entry, "file_count")?)?,
        total_bytes: as_u64(need(entry, "total_bytes")?)?,
        inventory_sha256: inventory_sha256.to_owned(),
    })
}

fn parse_status(value: &json::Json) -> VResult<Status> {
    match as_str(value)? {
        "complete" => Ok(Status::Complete),
        "incomplete" => Ok(Status::Incomplete),
        "failed" => Ok(Status::Failed),
        _ => Err(Invalid),
    }
}

fn members<'a>(
    value: &'a json::Json,
    required: &[&str],
    optional: &[&str],
) -> VResult<&'a [(String, json::Json)]> {
    let json::Json::Object(entries) = value else {
        return Err(Invalid);
    };
    for (key, _) in entries {
        if !required.contains(&key.as_str()) && !optional.contains(&key.as_str()) {
            return Err(Invalid);
        }
    }
    for key in required {
        if !entries.iter().any(|(name, _)| name.as_str() == *key) {
            return Err(Invalid);
        }
    }
    Ok(entries)
}

fn get<'a>(entries: &'a [(String, json::Json)], key: &str) -> Option<&'a json::Json> {
    entries
        .iter()
        .find(|(name, _)| name.as_str() == key)
        .map(|(_, value)| value)
}

fn need<'a>(entries: &'a [(String, json::Json)], key: &str) -> VResult<&'a json::Json> {
    get(entries, key).ok_or(Invalid)
}

fn as_u64(value: &json::Json) -> VResult<u64> {
    match value {
        json::Json::Number(number) if *number >= 0 => Ok(*number as u64),
        _ => Err(Invalid),
    }
}

fn as_i64(value: &json::Json) -> VResult<i64> {
    match value {
        json::Json::Number(number) => Ok(*number),
        _ => Err(Invalid),
    }
}

fn as_str(value: &json::Json) -> VResult<&str> {
    match value {
        json::Json::String(text) => Ok(text),
        _ => Err(Invalid),
    }
}

fn as_bool(value: &json::Json) -> VResult<bool> {
    value.as_bool().ok_or(Invalid)
}

fn nonempty_str(value: &json::Json) -> VResult<()> {
    if as_str(value)?.is_empty() {
        Err(Invalid)
    } else {
        Ok(())
    }
}

fn one_of_str<'a>(value: &'a json::Json, allowed: &[&str]) -> VResult<&'a str> {
    let text = as_str(value)?;
    if allowed.contains(&text) {
        Ok(text)
    } else {
        Err(Invalid)
    }
}

fn const_u64(value: &json::Json, expected: u64) -> VResult<()> {
    if as_u64(value)? == expected {
        Ok(())
    } else {
        Err(Invalid)
    }
}

fn const_str(value: &json::Json, expected: &str) -> VResult<()> {
    if as_str(value)? == expected {
        Ok(())
    } else {
        Err(Invalid)
    }
}

/// Schema-shaped simple table/artifact name `^[A-Za-z][A-Za-z0-9_]{0,79}$`.
fn valid_simple_name(value: &str) -> bool {
    let bytes = value.as_bytes();
    !bytes.is_empty()
        && bytes.len() <= 80
        && bytes[0].is_ascii_alphabetic()
        && bytes
            .iter()
            .all(|byte| byte.is_ascii_alphanumeric() || *byte == b'_')
}

/// Schema-shaped published path `^[A-Za-z][A-Za-z0-9_]{0,79}\.db2$`.
fn valid_relative_path(value: &str) -> bool {
    value.strip_suffix(".db2").is_some_and(valid_simple_name)
}

fn valid_hex(value: &str, length: usize) -> bool {
    value.len() == length
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn valid_lower_code(value: &str) -> bool {
    let bytes = value.as_bytes();
    !bytes.is_empty()
        && bytes[0].is_ascii_lowercase()
        && bytes
            .iter()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || *byte == b'_')
}

fn valid_uuid(value: &str) -> bool {
    let bytes = value.as_bytes();
    if bytes.len() != 36 {
        return false;
    }
    for (at, byte) in bytes.iter().enumerate() {
        if [8usize, 13, 18, 23].contains(&at) {
            if *byte != b'-' {
                return false;
            }
        } else if !byte.is_ascii_lowercase() && !byte.is_ascii_digit() {
            return false;
        }
    }
    matches!(bytes[14], b'1'..=b'5') && matches!(bytes[19], b'8' | b'9' | b'a' | b'b')
}

fn valid_semver(value: &str) -> bool {
    let (core, suffix) = match value.find(['-', '+']) {
        Some(at) => (&value[..at], Some(&value[at..])),
        None => (value, None),
    };
    let mut parts = core.split('.');
    let (Some(major), Some(minor), Some(patch), None) =
        (parts.next(), parts.next(), parts.next(), parts.next())
    else {
        return false;
    };
    if !digits_only(major) || !digits_only(minor) || !digits_only(patch) {
        return false;
    }
    if let Some(suffix) = suffix {
        let body = &suffix[1..];
        if body.is_empty()
            || !body
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || byte == b'.' || byte == b'-')
        {
            return false;
        }
    }
    true
}

fn digits_only(value: &str) -> bool {
    !value.is_empty() && value.bytes().all(|byte| byte.is_ascii_digit())
}

/// Bounded RFC3339 check for the emitted `YYYY-MM-DDTHH:MM:SSZ` shape plus an
/// optional fractional second and `Z`/`+HH:MM`/`-HH:MM` offset. It validates
/// field ranges; it is not a full calendrical validator.
fn valid_datetime(value: &str) -> bool {
    let bytes = value.as_bytes();
    if bytes.len() < 20
        || bytes[4] != b'-'
        || bytes[7] != b'-'
        || bytes[10] != b'T'
        || bytes[13] != b':'
        || bytes[16] != b':'
    {
        return false;
    }
    for range in [0..4usize, 5..7, 8..10, 11..13, 14..16, 17..19] {
        if !bytes[range].iter().all(u8::is_ascii_digit) {
            return false;
        }
    }
    let number = |range: std::ops::Range<usize>| value[range].parse::<u32>().ok();
    let (Some(_year), Some(month), Some(day), Some(hour), Some(minute), Some(second)) = (
        number(0..4),
        number(5..7),
        number(8..10),
        number(11..13),
        number(14..16),
        number(17..19),
    ) else {
        return false;
    };
    if !(1..=12).contains(&month)
        || !(1..=31).contains(&day)
        || hour > 23
        || minute > 59
        || second > 60
    {
        return false;
    }
    let mut rest = &bytes[19..];
    if rest.first() == Some(&b'.') {
        let mut at = 1;
        while at < rest.len() && rest[at].is_ascii_digit() {
            at += 1;
        }
        if at == 1 {
            return false;
        }
        rest = &rest[at..];
    }
    match rest {
        [b'Z'] => true,
        [sign, h1, h2, b':', m1, m2]
            if (*sign == b'+' || *sign == b'-')
                && h1.is_ascii_digit()
                && h2.is_ascii_digit()
                && m1.is_ascii_digit()
                && m2.is_ascii_digit() =>
        {
            let hours = u32::from(h1 - b'0') * 10 + u32::from(h2 - b'0');
            let minutes = u32::from(m1 - b'0') * 10 + u32::from(m2 - b'0');
            hours <= 23 && minutes <= 59
        }
        _ => false,
    }
}
