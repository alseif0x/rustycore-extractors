// SPDX-License-Identifier: GPL-3.0-or-later
//! Deterministic serializers for reviewed v2 and explicit catalog v3 manifests.
//!
//! This module owns no I/O, no storage and no client access: the batch feeds it
//! already-reviewed per-entry facts and it emits JSON in the exact field order
//! of their separate versioned schemas. The v2 byte format remains unchanged.
//! It uses `std` only (no JSON or
//! hashing dependency); `sha256` is the in-tree FIPS 180-4 implementation.
//!
//! Honest omissions: the batch has no source for a real `run_id` or an install
//! fingerprint (`admission::admit` exposes only the build), and the runtime has
//! no embedded git commit. Those fields are therefore optional in the schema
//! and omitted here rather than invented.
use crate::sha256;
use std::time::{SystemTime, UNIX_EPOCH};

/// Compatibility oracle pinned by the reviewed metadata transcription.
const ORACLE_COMMIT: &str = "02245dcd245e7433e524577656177723d3e4992e";
/// Release line for this extractor; kept in sync with the workspace version.
const EXTRACTOR_VERSION: &str = env!("CARGO_PKG_VERSION");
/// Documented batch limits (mirror `batch::{MAX_FILE_BYTES, MAX_BATCH_BYTES}`).
const MAX_FILE_BYTES: u64 = 16 * 1024 * 1024;
const MAX_BATCH_BYTES: u64 = 256 * 1024 * 1024;
/// Digest of the complete, reproducible factual catalog artifact. Version3
/// locks source declarations and their generated representation independently.
pub const CATALOG_FACTS_SHA256: &str =
    "0f1ed0c419e851078d99f46ad3b37cadddb27b0556a2d1b52d3fd9ab62b76ec9";
pub const CATALOG_SCOPE: &str = "explicit_db2_catalog_batch";

/// Identity and independent facts for one requested entry, in plan order.
///
/// On success (`error_code.is_none()`), `bytes` and `sha256` describe the
/// published `.db2`. On failure the metadata-derived facts survive when the
/// file was read and `Metadata::read` succeeded; the error row never claims a
/// decoded body.
pub struct TableFact {
    pub name: String,
    pub file_id: u32,
    pub table_hash: u32,
    pub layout_hash: u32,
    pub meta_index_field: i16,
    pub field_count: u32,
    /// Bytes read for this entry, present once `Metadata::read` succeeded.
    pub bytes: Option<usize>,
    /// Hex SHA-256 of the published `.db2`, present only on success.
    pub sha256: Option<String>,
    pub declared_records: Option<u32>,
    pub schema_validated: Option<bool>,
    /// `Some` only when the body was fully decoded.
    pub decoded_rows: Option<usize>,
    /// `None` on success; the static failure code otherwise.
    pub error_code: Option<&'static str>,
}

/// Batch-level options the manifest must disclose.
pub struct Options {
    pub locale: &'static str,
    pub table_selection: &'static str,
    pub manifest_sha256: Option<String>,
    pub entry_count: usize,
}

/// Build the manifest JSON. `status` is `complete` only when every entry
/// published; `reason` is present exactly when it is not.
pub fn render(
    build: u32,
    options: &Options,
    facts: &[TableFact],
    started_at: &str,
    completed_at: Option<&str>,
) -> String {
    render_version(build, options, facts, started_at, completed_at, false, None)
}

/// Separate v3 contract for explicitly selected pinned catalog schemas.
pub fn render_catalog(
    build: u32,
    options: &Options,
    facts: &[TableFact],
    started_at: &str,
    completed_at: Option<&str>,
) -> String {
    render_version(build, options, facts, started_at, completed_at, true, None)
}

/// Version 4 records explicit acquisition, independently of schema selection.
pub fn render_acquired(
    build: u32,
    options: &Options,
    facts: &[TableFact],
    started_at: &str,
    completed_at: Option<&str>,
    schema_catalog: bool,
    acquisition: crate::acquisition::Profile,
) -> String {
    render_version(
        build,
        options,
        facts,
        started_at,
        completed_at,
        schema_catalog,
        Some(acquisition),
    )
}

fn render_version(
    build: u32,
    options: &Options,
    facts: &[TableFact],
    started_at: &str,
    completed_at: Option<&str>,
    schema_catalog: bool,
    acquisition: Option<crate::acquisition::Profile>,
) -> String {
    let complete = facts.iter().all(|fact| fact.error_code.is_none());
    let file_count = facts
        .iter()
        .filter(|fact| fact.error_code.is_none())
        .count();
    let total_bytes: u64 = facts
        .iter()
        .filter(|fact| fact.error_code.is_none())
        .map(|fact| fact.bytes.unwrap_or(0) as u64)
        .sum();
    let inventory = inventory(facts);
    let inventory_sha256 = sha256::hex(inventory.as_bytes());

    let status = if complete { "complete" } else { "failed" };
    let mut out = String::with_capacity(4096);
    out.push_str(if acquisition.is_some() {
        "{\"manifest_version\":4"
    } else if schema_catalog {
        "{\"manifest_version\":3"
    } else {
        "{\"manifest_version\":2"
    });
    out.push_str(&format!(",\"status\":\"{status}\""));
    if !complete {
        out.push_str(&format!(
            ",\"reason\":\"{}\"",
            escape(&failure_reason(facts))
        ));
    }
    out.push_str(&format!(
        ",\"compatibility\":{{\"support\":\"admitted\",\"target_build\":{build}}}"
    ));
    out.push_str(&format!(
        ",\"client\":{{\"product\":\"wow_classic_beta\",\"version\":\"1.60.1.{build}\",\"build\":{build},\"locale\":\"{}\"}}",
        options.locale
    ));
    out.push_str(&format!(
        ",\"extractor\":{{\"name\":\"forever-db2-extract\",\"version\":\"{}\",\"implementation\":\"rust\",\"compatibility_oracle_commit\":\"{ORACLE_COMMIT}\"}}",
        escape(EXTRACTOR_VERSION)
    ));
    if schema_catalog {
        let catalog = forever_db2::catalog::CATALOG_ID;
        let commit = forever_db2::catalog::SOURCE_COMMIT;
        let blob = forever_db2::catalog::SOURCE_GIT_BLOB;
        let source_sha = forever_db2::catalog::SOURCE_SHA256;
        out.push_str(&format!(",\"scope\":\"{CATALOG_SCOPE}\",\"schema_catalog\":{{\"id\":\"{catalog}\",\"source_commit\":\"{commit}\",\"source_blob\":\"{blob}\",\"source_sha256\":\"{source_sha}\",\"facts_sha256\":\"{CATALOG_FACTS_SHA256}\"}}"));
    } else {
        out.push_str(",\"scope\":\"explicit_db2_batch\"");
    }
    if let Some(profile) = acquisition {
        out.push_str(&format!(
            ",\"schema_selection\":\"{}\",\"acquisition\":{}",
            if schema_catalog {
                "catalog"
            } else {
                "reviewed"
            },
            profile.render()
        ));
    }
    out.push_str(",\"formats\":{\"db2\":{\"magic\":\"WDC5\",\"version\":5},\"tables\":[");
    for (index, fact) in facts.iter().enumerate() {
        if index > 0 {
            out.push(',');
        }
        out.push_str(&format!(
            "{{\"name\":\"{}\",\"file_id\":{},\"table_hash\":{},\"layout_hash\":{},\"meta_index_field\":{},\"field_count\":{}}}",
            escape(&fact.name),
            fact.file_id,
            fact.table_hash,
            fact.layout_hash,
            fact.meta_index_field,
            fact.field_count
        ));
    }
    out.push_str("]}");
    let selector = if schema_catalog {
        format!(
            ",\"schema_catalog\":\"{}\"",
            forever_db2::catalog::CATALOG_ID
        )
    } else {
        String::new()
    };
    out.push_str(&format!(
        ",\"options\":{{\"locale\":\"{}\",\"table_selection\":\"{}\",\"manifest_sha256\":{},\"entry_count\":{},\"max_file_bytes\":{},\"max_batch_bytes\":{}{selector}}}",
        options.locale,
        escape(options.table_selection),
        options
            .manifest_sha256
            .as_deref()
            .map_or_else(|| "null".to_owned(), |value| format!("\"{value}\"")),
        options.entry_count,
        acquisition.map_or(MAX_FILE_BYTES, |profile| u64::from(profile.max_file_bytes())),
        MAX_BATCH_BYTES
    ));
    out.push_str(",\"network\":{\"mode\":\"local\",\"allow_network\":false,\"remote_casc\":false,\"lock_sha256\":null,\"downloaded_bytes\":0}");
    out.push_str(&format!(
        ",\"outputs\":{{\"root\":\".\",\"incomplete_marker_present\":{},\"artifacts\":[",
        !complete
    ));
    out.push_str(&render_artifacts(facts));
    out.push_str(&format!(
        "],\"integrity\":{{\"algorithm\":\"sha256\",\"file_count\":{file_count},\"total_bytes\":{total_bytes},\"inventory_sha256\":\"{inventory_sha256}\"}}}}"
    ));
    out.push_str(&format!(
        ",\"timestamps\":{{\"started_at\":\"{}\",\"completed_at\":{}}}",
        escape(started_at),
        completed_at.map_or_else(
            || "null".to_owned(),
            |value| format!("\"{}\"", escape(value))
        )
    ));
    out.push_str(",\"privacy\":{\"paths_redacted\":true,\"client_data_embedded\":false}}");
    out
}

fn render_artifacts(facts: &[TableFact]) -> String {
    let mut out = String::new();
    for fact in facts {
        if !out.is_empty() {
            out.push(',');
        }
        let name = escape(&fact.name);
        out.push_str(&format!(
            "{{\"name\":\"{name}\",\"file_id\":{},\"relative_path\":\"{name}.db2\",",
            fact.file_id
        ));
        match fact.error_code {
            Some(code) => out.push_str(&format!(
                "\"status\":\"error\",\"code\":\"{}\",\"bytes\":{},\"sha256\":\"{}\",\"declared_records\":{},\"table_schema_validated\":{},\"record_data_decoded\":false,\"decoded_rows\":null}}",
                escape(code),
                fact.bytes.unwrap_or(0),
                sha256::hex(b""),
                fact.declared_records.unwrap_or(0),
                fact.schema_validated.unwrap_or(false)
            )),
            None => out.push_str(&format!(
                "\"status\":\"complete\",\"bytes\":{},\"sha256\":\"{}\",\"declared_records\":{},\"table_schema_validated\":{},\"record_data_decoded\":{},\"decoded_rows\":{}}}",
                fact.bytes.unwrap_or(0),
                fact.sha256.clone().unwrap_or_else(|| sha256::hex(b"")),
                fact.declared_records.unwrap_or(0),
                fact.schema_validated.unwrap_or(false),
                fact.decoded_rows.is_some(),
                fact.decoded_rows
                    .map_or_else(|| "null".to_owned(), |rows| rows.to_string())
            )),
        }
    }
    out
}

fn inventory(facts: &[TableFact]) -> String {
    let mut lines: Vec<String> = facts
        .iter()
        .filter(|fact| fact.error_code.is_none())
        .map(|fact| {
            format!(
                "{}  {}.db2",
                fact.sha256.as_deref().unwrap_or(""),
                fact.name
            )
        })
        .collect();
    // Sort by the UTF-8 bytes of the whole line so the order is stable across
    // locales and independent of plan order.
    lines.sort_by(|left, right| left.as_bytes().cmp(right.as_bytes()));
    lines.join("\n")
}

fn failure_reason(facts: &[TableFact]) -> String {
    let codes: Vec<&str> = facts.iter().filter_map(|fact| fact.error_code).collect();
    if codes.is_empty() {
        "batch completed with an unpublished artifact".to_owned()
    } else {
        format!(
            "{} of {} requested tables failed: {}",
            codes.len(),
            facts.len(),
            codes.join(",")
        )
    }
}

/// RFC3339 UTC timestamp with second precision and a `Z` suffix. The schema
/// requires `date-time`; no chrono dependency is available, so the civil date
/// is derived from Unix epoch seconds by the standard 400-year cycle algorithm.
pub fn now_rfc3339() -> String {
    let seconds = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|elapsed| elapsed.as_secs())
        .unwrap_or(0);
    rfc3339_from_unix(seconds)
}

/// Formats nonnegative Unix seconds as `YYYY-MM-DDTHH:MM:SSZ` (proleptic
/// Gregorian, UTC). Split out so tests can pin exact dates.
pub fn rfc3339_from_unix(seconds: u64) -> String {
    let days = (seconds / 86_400) as i64;
    let second_of_day = seconds % 86_400;
    let (year, month, day) = civil_from_days(days);
    format!(
        "{year:04}-{month:02}-{day:02}T{:02}:{:02}:{:02}Z",
        second_of_day / 3600,
        (second_of_day % 3600) / 60,
        second_of_day % 60
    )
}

/// Howard Hinnant's `civil_from_days`: days since 1970-01-01 to (year, month, day).
fn civil_from_days(days: i64) -> (i64, u32, u32) {
    let shifted = days + 719_468;
    let era = if shifted >= 0 {
        shifted
    } else {
        shifted - 146_096
    } / 146_097;
    let day_of_era = shifted - era * 146_097; // [0, 146096]
    let year_of_era =
        (day_of_era - day_of_era / 1460 + day_of_era / 36_524 - day_of_era / 146_096) / 365; // [0, 399]
    let year = year_of_era + era * 400;
    let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100); // [0, 365]
    let month_prime = (5 * day_of_year + 2) / 153; // [0, 11]
    let day = (day_of_year - (153 * month_prime + 2) / 5 + 1) as u32; // [1, 31]
    let month = (month_prime + if month_prime < 10 { 3 } else { -9 }) as u32; // [1, 12]
    let year = year + i64::from(month <= 2);
    (year, month, day)
}

/// Minimal JSON string escaping for the ASCII table names, locale, version and
/// static codes. Sufficient for the manifest's fixed, validated ASCII fields.
fn escape(value: &str) -> String {
    let mut out = String::with_capacity(value.len());
    for character in value.chars() {
        match character {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            character if character.is_control() => {
                out.push_str(&format!("\\u{:04x}", character as u32))
            }
            character => out.push(character),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn epoch_is_formatted_as_rfc3339() {
        assert_eq!("1970-01-01T00:00:00Z", rfc3339_from_unix(0));
        assert_eq!("2026-10-04T10:00:00Z", rfc3339_from_unix(1_791_108_000));
        // Leap day handling.
        assert_eq!("2024-02-29T12:34:56Z", rfc3339_from_unix(1_709_210_096));
    }

    #[test]
    fn catalog_provenance_digest_matches_committed_fact_bytes() {
        assert_eq!(
            CATALOG_FACTS_SHA256,
            sha256::hex(include_bytes!(
                "../../../contracts/forever-metadata-v1.json"
            ))
        );
    }

    #[test]
    fn v2_bytes_remain_identical_when_catalog_is_not_selected() {
        let options = Options {
            locale: "enUS",
            table_selection: "default_character_tables",
            manifest_sha256: None,
            entry_count: 1,
        };
        let facts = [fact("ChrClasses", 1361031, None)];
        let start = "2026-10-11T00:00:00Z";
        let document = render(70205, &options, &facts, start, Some(start));
        assert!(document.starts_with("{\"manifest_version\":2,"));
        assert!(document.contains("\"scope\":\"explicit_db2_batch\""));
        assert!(!document.contains("schema_catalog"));
        assert_eq!(
            document,
            include_str!(
                "../../../tests/contracts/fixtures/forever-manifest-v2.serializer-golden.json"
            )
            .trim_end()
        );
        assert_eq!(
            document,
            render_version(70205, &options, &facts, start, Some(start), false, None)
        );
        let explicit = Options {
            table_selection: "explicit_manifest",
            manifest_sha256: Some("a".repeat(64)),
            ..options
        };
        let catalog = render_catalog(70205, &explicit, &facts, start, Some(start));
        assert!(catalog.starts_with("{\"manifest_version\":3,"));
        assert!(catalog.contains("\"scope\":\"explicit_db2_catalog_batch\""));
        assert!(catalog.contains(CATALOG_FACTS_SHA256));
    }

    #[test]
    fn inventory_is_sorted_by_utf8_bytes_and_skips_error_rows() {
        let facts = vec![
            fact("ChrRaces", 1305311, None),
            fact("ChrClasses", 1361031, None),
            TableFact {
                name: "Alpha".to_owned(),
                file_id: 1,
                table_hash: 0,
                layout_hash: 0,
                meta_index_field: 0,
                field_count: 1,
                bytes: None,
                sha256: None,
                declared_records: None,
                schema_validated: None,
                decoded_rows: None,
                error_code: Some("file_unavailable"),
            },
        ];
        let rendered = inventory(&facts);
        assert!(!rendered.contains("Alpha"));
        // Lines are ordered by the UTF-8 bytes of the whole `digest  path` line,
        // not by table name, so compare against the explicit sorted expectation.
        let mut expected = [
            format!("{}  ChrClasses.db2", sha("ChrClasses")),
            format!("{}  ChrRaces.db2", sha("ChrRaces")),
        ];
        expected.sort_by(|left, right| left.as_bytes().cmp(right.as_bytes()));
        assert_eq!(expected.join("\n"), rendered);
    }

    fn fact(name: &str, file_id: u32, error: Option<&'static str>) -> TableFact {
        let bytes = if error.is_none() { Some(32) } else { None };
        TableFact {
            name: name.to_owned(),
            file_id,
            table_hash: 0,
            layout_hash: 0,
            meta_index_field: 0,
            field_count: 1,
            bytes,
            sha256: error.is_none().then(|| sha(name)),
            declared_records: Some(1),
            schema_validated: Some(true),
            decoded_rows: Some(1),
            error_code: error,
        }
    }

    fn sha(name: &str) -> String {
        sha256::hex(name.as_bytes())
    }
}
