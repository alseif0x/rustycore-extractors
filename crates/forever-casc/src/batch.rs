// SPDX-License-Identifier: GPL-3.0-or-later
//! Explicit-list local DB2 extraction. Never scan/download a remote listfile.
mod output;
#[cfg(test)]
pub(crate) mod tests;
use crate::Error;
use crate::manifest::TableFact;
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
    /// True when the operator supplied an explicit manifest file; false for the
    /// reviewed default character-table plan. Feeds the v2 `table_selection`.
    explicit: bool,
    /// SHA-256 of the exact manifest text, present only for explicit plans.
    manifest_sha256: Option<String>,
    /// Present only after every entry resolved in this exact catalog with its
    /// exact spelling and FileDataId. v2 remains unchanged.
    schema_catalog: Option<&'static str>,
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
            explicit: false,
            manifest_sha256: None,
            schema_catalog: None,
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
        Ok(Self {
            entries,
            explicit: true,
            manifest_sha256: Some(crate::sha256::hex(input.as_bytes())),
            schema_catalog: None,
        })
    }

    /// Explicit v3 catalog plan. Unknown/case-mismatched names and wrong IDs
    /// fail before output creation; there is no metadata-only catalog fallback.
    pub fn parse_with_catalog(input: &str, catalog: &'static str) -> Result<Self, &'static str> {
        if !forever_db2::catalog::supports(catalog) {
            return Err("invalid_manifest");
        }
        let mut plan = Self::parse(input)?;
        for entry in &plan.entries {
            let table = CharacterTable::from_catalog_id(catalog, &entry.name)
                .map_err(|_| "invalid_manifest")?
                .ok_or("invalid_manifest")?;
            if entry.id != table.file_id() {
                return Err("invalid_manifest");
            }
        }
        plan.schema_catalog = Some(catalog);
        Ok(plan)
    }

    fn table(&self, name: &str) -> Result<Option<CharacterTable>, &'static str> {
        if let Some(catalog) = self.schema_catalog {
            CharacterTable::from_catalog_id(catalog, name)
                .map_err(|_| "invalid_manifest")?
                .map(Some)
                .ok_or("invalid_manifest")
        } else {
            Ok(CharacterTable::named(name))
        }
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
        decoded_rows: Option<usize>,
    },
}
impl Outcome {
    fn failed(code: &'static str) -> Self {
        Self::Failed {
            code,
            bytes: None,
            declared_records: None,
            schema_validated: None,
            decoded_rows: None,
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
                ..
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
    /// Frozen per-entry facts feeding the v2 manifest. Complete rows carry the
    /// published-bytes digest; failed rows keep whatever metadata facts survived.
    fn fact(&self, entry: &Entry, fact: FactIdentity, digest: Option<String>) -> TableFact {
        match self {
            Self::Complete {
                bytes,
                declared_records,
                schema_validated,
                decoded_rows,
            } => TableFact {
                name: entry.name.clone(),
                file_id: entry.id,
                table_hash: fact.table_hash,
                layout_hash: fact.layout_hash,
                meta_index_field: fact.meta_index_field,
                field_count: fact.field_count,
                bytes: Some(*bytes),
                sha256: digest,
                declared_records: Some(*declared_records),
                schema_validated: Some(*schema_validated),
                decoded_rows: *decoded_rows,
                error_code: None,
            },
            Self::Failed {
                code,
                bytes,
                declared_records,
                schema_validated,
                decoded_rows,
            } => TableFact {
                name: entry.name.clone(),
                file_id: entry.id,
                table_hash: fact.table_hash,
                layout_hash: fact.layout_hash,
                meta_index_field: fact.meta_index_field,
                field_count: fact.field_count,
                bytes: *bytes,
                sha256: None,
                declared_records: *declared_records,
                schema_validated: *schema_validated,
                decoded_rows: *decoded_rows,
                error_code: Some(code),
            },
        }
    }
}

/// Header identity for one entry; `field_count >= 1` even when metadata never
/// parsed, because the schema requires a positive field count.
///
/// `meta_index_field` is the reviewed metadata authority for inline vs id-table
/// decoding, NOT the native header `IndexField` word. Real 70205 evidence shows
/// the two disagree (e.g. `ChrClassesXPowerTypes` header `index_field == 0`
/// while the reviewed meta index is `-1`), so the header word must never be
/// published as `meta_index_field`. When the table is not reviewed there is no
/// reviewed authority and the honest sentinel is `0`.
#[derive(Clone, Copy)]
struct FactIdentity {
    table_hash: u32,
    layout_hash: u32,
    meta_index_field: i16,
    field_count: u32,
}
impl FactIdentity {
    fn unknown() -> Self {
        Self {
            table_hash: 0,
            layout_hash: 0,
            meta_index_field: 0,
            field_count: 1,
        }
    }
    /// Header identity plus the reviewed `meta_index_field`. The header supplies
    /// table/layout hash and field count; the reviewed schema supplies the meta
    /// index (sentinel `0` when `known` is `None`).
    fn from(known: Option<CharacterTable>, metadata: &Metadata) -> Self {
        Self {
            table_hash: metadata.header.table_hash,
            layout_hash: metadata.header.layout_hash,
            meta_index_field: known.map_or(0, |table| table.schema_identity().2),
            field_count: metadata.header.field_count,
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
    load: impl FnMut(&Entry, u32) -> Result<(u32, Vec<u8>), Error>,
) -> Result<Report, &'static str> {
    extract_with_acquisition(plan, client, destination, build, locale, None, load)
}

/// Version 4 when an explicit Rust-local acquisition profile is supplied.
/// The profile changes acquisition and resource limits, not table selection or
/// the existing distinction between reviewed records and metadata-only entries.
/// The caller opens one Session before invoking this output-producing function.
pub fn extract_with_acquisition(
    plan: &Plan,
    client: &Path,
    destination: &Path,
    build: u32,
    locale: &str,
    acquisition: Option<crate::acquisition::Profile>,
    mut load: impl FnMut(&Entry, u32) -> Result<(u32, Vec<u8>), Error>,
) -> Result<Report, &'static str> {
    if !crate::SUPPORTED_CLIENTS
        .iter()
        .any(|(_, supported)| *supported == build)
        || !matches!(locale, "enUS" | "esES")
    {
        return Err("unsupported_client");
    }
    if plan.schema_catalog == Some(forever_db2::catalog::WOWDBDEFS_70205_CATALOG_ID)
        && build != 70205
    {
        return Err("unsupported_client");
    }
    if plan.schema_catalog == Some(forever_db2::catalog::WOWDBDEFS_70338_CATALOG_ID)
        && build != 70338
    {
        return Err("unsupported_client");
    }
    let max_file_bytes = acquisition.map_or(MAX_FILE_BYTES, |profile| profile.max_file_bytes());
    if max_file_bytes == 0 || max_file_bytes > MAX_FILE_BYTES {
        return Err("read_policy_limit");
    }
    let directory = output::create(client, destination)?;
    if let Some(profile) = acquisition {
        let marker = format!(
            "{{\"manifest_version\":4,\"status\":\"incomplete\",\"acquisition\":{},\"paths_redacted\":true}}",
            profile.render()
        );
        output::publish(
            &directory,
            ".rustycore-forever-incomplete",
            "json",
            marker.as_bytes(),
        )?;
    }
    let started_at = crate::manifest::now_rfc3339();
    let mut total = 0u64;
    let mut complete = true;
    let mut reports = Vec::new();
    let mut facts = Vec::new();
    for entry in &plan.entries {
        let remaining = MAX_BATCH_BYTES - total;
        let budget = remaining.min(u64::from(max_file_bytes)) as u32;
        let known = plan.table(&entry.name)?;
        let identity = std::cell::Cell::new(FactIdentity::unknown());
        let digest = std::cell::Cell::new(None);
        let outcome = if budget == 0 {
            Outcome::failed("batch_policy_limit")
        } else {
            match load(entry, budget) {
                Err(error) if acquisition.is_some() && error.is_input_changed() => {
                    // Fatal: no more reads or publication; the real marker stays.
                    return Err(error.code());
                }
                Err(error) => Outcome::failed(error.code()),
                Ok((observed, bytes)) => {
                    if observed != build {
                        if acquisition.is_some() {
                            return Err("client_build_changed");
                        }
                        Outcome::failed("client_build_changed")
                    } else if bytes.len() > budget as usize {
                        Outcome::failed("read_policy_limit")
                    } else {
                        // Count every successfully returned buffer, including rejected DB2.
                        total += bytes.len() as u64;
                        match Metadata::read(&mut Cursor::new(&bytes)) {
                            Err(error) => Outcome::failed(error.code()),
                            Ok(metadata) => {
                                identity.set(FactIdentity::from(known, &metadata));
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
                                        decoded_rows: None,
                                    },
                                    Ok(decoded_rows) => {
                                        match output::publish(
                                            &directory,
                                            &entry.name,
                                            "db2",
                                            &bytes,
                                        ) {
                                            Err(code) => Outcome::failed(code),
                                            Ok(()) => {
                                                digest.set(Some(crate::sha256::hex(&bytes)));
                                                Outcome::Complete {
                                                    bytes: bytes.len(),
                                                    declared_records: metadata.header.record_count,
                                                    schema_validated,
                                                    decoded_rows: decoded_rows
                                                        .map(|table| table.rows().len()),
                                                }
                                            }
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
        facts.push(outcome.fact(entry, identity.get(), digest.take()));
    }
    let scope = if plan.schema_catalog.is_some() {
        "explicit_db2_catalog_batch"
    } else {
        "explicit_db2_batch"
    };
    let catalog_report = if let Some(catalog) = plan.schema_catalog {
        format!(",\"schema_catalog\":\"{}\"", catalog)
    } else {
        String::new()
    };
    let acquisition_report = acquisition.map_or_else(String::new, |profile| {
        format!(
            ",\"schema_selection\":\"{}\",\"acquisition\":{}",
            if plan.schema_catalog.is_some() {
                "catalog"
            } else {
                "reviewed"
            },
            profile.render()
        )
    });
    let report_version = if acquisition.is_some() { 2 } else { 1 };
    let json = format!(
        "{{\"report_version\":{report_version},\"scope\":\"{scope}\"{catalog_report}{acquisition_report},\"status\":\"{}\",\"build\":{},\"locale\":\"{}\",\"offline\":true,\"paths_redacted\":true,\"all_requested_files_complete\":{},\"full_client_extraction\":false,\"files\":[{}]}}",
        if complete { "success" } else { "incomplete" },
        build,
        locale,
        complete,
        reports.join(",")
    );
    let options = crate::manifest::Options {
        locale: if locale == "enUS" { "enUS" } else { "esES" },
        table_selection: if plan.explicit {
            "explicit_manifest"
        } else {
            "default_character_tables"
        },
        manifest_sha256: plan.manifest_sha256.clone(),
        entry_count: plan.entries.len(),
    };
    let completed_at = crate::manifest::now_rfc3339();
    let manifest = if let Some(profile) = acquisition {
        crate::manifest::render_acquired_for(
            build,
            &options,
            &facts,
            &started_at,
            Some(&completed_at),
            plan.schema_catalog,
            profile,
        )
    } else if let Some(catalog) = plan.schema_catalog {
        crate::manifest::render_catalog_for(
            build,
            &options,
            &facts,
            &started_at,
            Some(&completed_at),
            catalog,
        )
    } else {
        crate::manifest::render(build, &options, &facts, &started_at, Some(&completed_at))
    };
    // If either JSON document cannot be published, extraction returns Err and
    // the already published `.db2` files remain on disk without it. That
    // partial directory is explicit (the manifest is required for a complete
    // result); we neither silently delete files nor report success without it.
    // Publish `.db2` files first (done above), then `report.json` for
    // compatibility, then the v2 `forever-manifest.json`.
    output::publish(&directory, "report", "json", json.as_bytes())?;
    output::publish(&directory, "forever-manifest", "json", manifest.as_bytes())?;
    if complete && acquisition.is_some() {
        std::fs::remove_file(directory.join(crate::verify::MARKER_NAME))
            .map_err(|_| "output_write_failed")?;
    }
    Ok(Report { complete, json })
}
