// SPDX-License-Identifier: GPL-3.0-or-later
//! Explicit-list local DB2 extraction. Never scan/download a remote listfile.
mod output;
#[cfg(test)]
mod tests;
use crate::Error;
use forever_db2::{CharacterTable, DecodedTable, Metadata};
use std::{collections::BTreeSet, io::Cursor, path::Path};

pub const MAX_FILES: usize = 256;
pub const MAX_FILE_BYTES: u32 = 16 * 1024 * 1024;
pub const MAX_BATCH_BYTES: u64 = 256 * 1024 * 1024;
pub struct Entry {
    pub name: String,
    pub id: u32,
}
pub struct Plan {
    entries: Vec<Entry>,
}
impl Plan {
    /// Two reviewed tables, not an invented full-client catalog.
    pub fn character_tables() -> Self {
        Self {
            entries: [CharacterTable::Classes, CharacterTable::Races]
                .into_iter()
                .map(|table| Entry {
                    name: table.name().to_owned(),
                    id: table.file_id(),
                })
                .collect(),
        }
    }
    /// ASCII table name TAB decimal FileDataID; no paths, comments, keys or URLs.
    pub fn parse(input: &str) -> Result<Self, &'static str> {
        if input.len() > 64 * 1024 {
            return Err("manifest_policy_limit");
        }
        let mut names = BTreeSet::new();
        let mut ids = BTreeSet::new();
        let mut entries = Vec::new();
        for line in input.lines() {
            let (name, id) = line.split_once('\t').ok_or("invalid_manifest")?;
            if name.is_empty()
                || name.len() > 80
                || !name.as_bytes()[0].is_ascii_alphabetic()
                || !name
                    .bytes()
                    .all(|byte| byte.is_ascii_alphanumeric() || byte == b'_')
                || !id.bytes().all(|byte| byte.is_ascii_digit())
            {
                return Err("invalid_manifest");
            }
            let id = id.parse::<u32>().map_err(|_| "invalid_manifest")?;
            if id == 0
                || id == u32::MAX
                || !names.insert(name.to_ascii_lowercase())
                || !ids.insert(id)
            {
                return Err("invalid_manifest");
            }
            if let Some(table) = CharacterTable::named(name)
                && table.file_id() != id
            {
                return Err("invalid_manifest");
            }
            entries.push(Entry {
                name: name.to_owned(),
                id,
            });
            if entries.len() > MAX_FILES {
                return Err("manifest_policy_limit");
            }
        }
        if entries.is_empty() {
            return Err("invalid_manifest");
        }
        Ok(Self { entries })
    }
}
pub struct Report {
    pub complete: bool,
    pub json: String,
}
/// Per-entry result. Failure rows keep the independent schema/declared facts
/// whenever the file was fully read and its metadata parsed, so a decode failure
/// does not collapse schema-vs-decode into a single opaque code.
enum Outcome {
    Complete {
        bytes: usize,
        declared_records: u32,
        schema_validated: bool,
        decoded_rows: Option<usize>,
    },
    Failed {
        code: &'static str,
        bytes: Option<usize>,
        declared_records: Option<u32>,
        schema_validated: Option<bool>,
    },
}
impl Outcome {
    fn failed(code: &'static str) -> Self {
        Self::Failed {
            code,
            bytes: None,
            declared_records: None,
            schema_validated: None,
        }
    }
    fn is_complete(&self) -> bool {
        matches!(self, Self::Complete { .. })
    }
    fn json(&self, entry: &Entry) -> String {
        match self {
            Self::Complete {
                bytes,
                declared_records,
                schema_validated,
                decoded_rows,
            } => format!(
                "{{\"name\":\"{}\",\"file_id\":{},\"status\":\"complete\",\"bytes\":{},\"declared_records\":{},\"table_schema_validated\":{},\"record_data_decoded\":{},\"decoded_rows\":{}}}",
                entry.name,
                entry.id,
                bytes,
                declared_records,
                schema_validated,
                decoded_rows.is_some(),
                decoded_rows.map_or_else(|| "null".to_owned(), |rows| rows.to_string())
            ),
            Self::Failed {
                code,
                bytes: Some(bytes),
                declared_records: Some(declared_records),
                schema_validated: Some(schema_validated),
            } => format!(
                "{{\"name\":\"{}\",\"file_id\":{},\"status\":\"error\",\"code\":\"{}\",\"bytes\":{},\"declared_records\":{},\"table_schema_validated\":{}}}",
                entry.name, entry.id, code, bytes, declared_records, schema_validated
            ),
            Self::Failed { code, .. } => format!(
                "{{\"name\":\"{}\",\"file_id\":{},\"status\":\"error\",\"code\":\"{}\"}}",
                entry.name, entry.id, code
            ),
        }
    }
}
/// `load` must return a complete strict file and the independently matched build.
/// Admission and the complete plan precede any output creation. This library
/// seam permits synthetic failure tests without native storage or client fixtures.
pub fn extract(
    plan: &Plan,
    client: &Path,
    destination: &Path,
    build: u32,
    locale: &str,
    mut load: impl FnMut(&Entry, u32) -> Result<(u32, Vec<u8>), Error>,
) -> Result<Report, &'static str> {
    if !crate::SUPPORTED_CLIENTS
        .iter()
        .any(|(_, supported)| *supported == build)
        || !matches!(locale, "enUS" | "esES")
    {
        return Err("unsupported_client");
    }
    let directory = output::create(client, destination)?;
    let mut total = 0u64;
    let mut complete = true;
    let mut reports = Vec::new();
    for entry in &plan.entries {
        let remaining = MAX_BATCH_BYTES - total;
        let budget = remaining.min(u64::from(MAX_FILE_BYTES)) as u32;
        let known = CharacterTable::named(&entry.name);
        let outcome = if budget == 0 {
            Outcome::failed("batch_policy_limit")
        } else {
            match load(entry, budget) {
                Err(error) => Outcome::failed(error.code()),
                Ok((observed, bytes)) => {
                    if observed != build {
                        Outcome::failed("client_build_changed")
                    } else if bytes.len() > budget as usize {
                        Outcome::failed("read_policy_limit")
                    } else {
                        // Count every successfully returned buffer, including rejected DB2.
                        total += bytes.len() as u64;
                        match Metadata::read(&mut Cursor::new(&bytes)) {
                            Err(error) => Outcome::failed(error.code()),
                            Ok(metadata) => {
                                // Schema identity is independent of body decode: it
                                // stays true even when records cannot be decoded.
                                let schema_validated =
                                    known.is_some_and(|table| table.matches(&metadata.header));
                                let decoded = match known {
                                    Some(table) => DecodedTable::read(&bytes, table)
                                        .map(Some)
                                        .map_err(|error| error.code()),
                                    None => Ok(None),
                                };
                                match decoded {
                                    Err(code) => Outcome::Failed {
                                        code,
                                        bytes: Some(bytes.len()),
                                        declared_records: Some(metadata.header.record_count),
                                        schema_validated: Some(schema_validated),
                                    },
                                    Ok(decoded_rows) => {
                                        match output::publish(&directory, &entry.name, &bytes) {
                                            Err(code) => Outcome::failed(code),
                                            Ok(()) => Outcome::Complete {
                                                bytes: bytes.len(),
                                                declared_records: metadata.header.record_count,
                                                schema_validated,
                                                decoded_rows: decoded_rows
                                                    .map(|table| table.rows().len()),
                                            },
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        };
        if !outcome.is_complete() {
            complete = false;
        }
        reports.push(outcome.json(entry));
    }
    let json = format!(
        "{{\"report_version\":1,\"scope\":\"explicit_db2_batch\",\"status\":\"{}\",\"build\":{},\"locale\":\"{}\",\"offline\":true,\"paths_redacted\":true,\"all_requested_files_complete\":{},\"full_client_extraction\":false,\"files\":[{}]}}",
        if complete { "success" } else { "incomplete" },
        build,
        locale,
        complete,
        reports.join(",")
    );
    // If the report cannot be published, extraction returns Err and the already
    // published `.db2` files remain on disk with no `report.json`. That partial
    // directory is explicit (a report is required for a complete result); we do
    // not silently delete files or report success without the manifest report.
    output::publish(&directory, "report", json.as_bytes())?;
    Ok(Report { complete, json })
}
