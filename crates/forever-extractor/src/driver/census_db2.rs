// SPDX-License-Identifier: GPL-3.0-or-later
//! Complete-catalogue keyless DB2 source availability census for Linux.
//! This is a diagnostic report only: it publishes no DB2 output or package and
//! never converts a partial keyless result into extraction/completeness proof.

use crate::{
    Error,
    plan::{Plan, SchemaCatalog},
};
use forever_casc::local_store::probe::{self, FileTrace};
use forever_db2::catalog;
use std::{
    fs,
    io::Write,
    os::unix::fs::OpenOptionsExt,
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
};

const PROFILE: &str = "forever-db2-keyless-availability-census-v1";
const MAX_DECLARATIONS: usize = 2048;
const MAX_CANDIDATE_SPAN_BYTES: u64 = 2 * 1024 * 1024 * 1024;
const MAX_PER_DECLARATION_BYTES: u64 = 16 * 1024 * 1024;
const MAX_REPORT_BYTES: usize = 32 * 1024 * 1024;
static NEXT_TEMP: AtomicU64 = AtomicU64::new(0);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum State {
    FullyReadable,
    EncryptedNoAvailableKey,
    FileUnavailable,
    UnsupportedOrUnknown,
}
impl State {
    const ALL: [Self; 4] = [
        Self::FullyReadable,
        Self::EncryptedNoAvailableKey,
        Self::FileUnavailable,
        Self::UnsupportedOrUnknown,
    ];
    const fn name(self) -> &'static str {
        match self {
            Self::FullyReadable => "fully_readable",
            Self::EncryptedNoAvailableKey => "encrypted_no_available_key",
            Self::FileUnavailable => "file_unavailable",
            Self::UnsupportedOrUnknown => "unsupported_or_unknown",
        }
    }
    const fn index(self) -> usize {
        match self {
            Self::FullyReadable => 0,
            Self::EncryptedNoAvailableKey => 1,
            Self::FileUnavailable => 2,
            Self::UnsupportedOrUnknown => 3,
        }
    }
}

#[derive(Clone, Copy)]
pub(crate) struct Summary {
    pub declaration_count: usize,
}

pub(crate) fn run(plan: &Plan, client: &Path, output: &Path) -> Result<Summary, Error> {
    super::admit(plan)?;
    if plan.build != 70450
        || plan.version() < 16
        || plan.schema_catalog != SchemaCatalog::WowDbDefs70450V1
    {
        return Err("product_probe_profile".into());
    }
    let output = super::probe_encoding::output_path(client, output)?;
    let store = super::probe_encoding::open_keyless_store_70450(plan, client)?;
    if store.build().build != 70450 {
        return Err("product_build_mismatch".into());
    }
    if store.build().tags.len() > MAX_REPORT_BYTES {
        return Err("product_census_limit".into());
    }

    // The catalogue is authoritative and ordered. Do not derive this list from
    // the plan's selected DB2 entries: the purpose is to account for all facts.
    let declarations = catalog::tables_for(catalog::WOWDBDEFS_70450_CATALOG_ID)
        .ok_or_else(|| Error::from("product_probe_profile"))?;
    if declarations.len() > MAX_DECLARATIONS {
        return Err("product_census_limit".into());
    }
    let declaration_count = declarations.len();
    let mut counts = [0usize; 4];
    let mut entries = Vec::new();
    entries
        .try_reserve_exact(declaration_count)
        .map_err(|_| Error::from("product_census_limit"))?;
    let mut retained_entry_bytes = 0usize;
    let mut candidate_bytes_considered = 0u64;

    for (ordinal, declaration) in declarations.iter().enumerate() {
        let remaining = MAX_CANDIDATE_SPAN_BYTES.saturating_sub(candidate_bytes_considered);
        let per_declaration_limit = remaining.min(MAX_PER_DECLARATION_BYTES);
        let mut limits = probe::Limits::default();
        limits.max_record_bytes = limits.max_record_bytes.min(per_declaration_limit);
        limits.max_total_record_bytes = limits.max_total_record_bytes.min(per_declaration_limit);

        let (state, state_reason, trace_json) =
            match store.probe_file_data_ids(&[declaration.file_data_id], limits) {
                Ok(report) => {
                    candidate_bytes_considered = candidate_bytes_considered
                        .checked_add(report.candidate_bytes_considered)
                        .ok_or_else(|| Error::from("product_census_limit"))?;
                    let Some(file) = report.files.first() else {
                        return Err("product_census_internal".into());
                    };
                    let (state, reason) = classify(file);
                    let trace = report.render().map_err(|error| Error::from(error.code()))?;
                    (state, reason, Some(trace))
                }
                Err(error) => {
                    // An error may follow earlier bounded candidate attempts. Charge
                    // the entire per-declaration allowance conservatively so later
                    // declarations cannot turn an error path into unbounded I/O.
                    candidate_bytes_considered = candidate_bytes_considered
                        .saturating_add(per_declaration_limit)
                        .min(MAX_CANDIDATE_SPAN_BYTES);
                    let (state, reason) =
                        if per_declaration_limit == 0 && error.code() == "probe_encoding_limit" {
                            (
                                State::UnsupportedOrUnknown,
                                "global_candidate_span_budget_exhausted",
                            )
                        } else {
                            classify_error(error.code())
                        };
                    (state, reason, None)
                }
            };
        counts[state.index()] = counts[state.index()]
            .checked_add(1)
            .ok_or_else(|| Error::from("product_census_limit"))?;

        let name = json_quote(declaration.name)?;
        let reason = json_quote(state_reason)?;
        let trace = trace_json.unwrap_or_else(|| "null".to_owned());
        let entry = format!(
            "{{\"declaration_ordinal\":{ordinal},\"table_name\":{name},\"file_data_id\":{},\"state\":\"{}\",\"state_reason\":{reason},\"trace\":{trace}}}",
            declaration.file_data_id,
            state.name(),
        );
        retained_entry_bytes = retained_entry_bytes
            .checked_add(entry.len())
            .ok_or_else(|| Error::from("product_census_limit"))?;
        if retained_entry_bytes > MAX_REPORT_BYTES {
            return Err("product_census_limit".into());
        }
        entries.push(entry);
    }

    store
        .verify_unchanged()
        .map_err(|error| Error::from(error.code()))?;
    let locale_mask = match plan.locale {
        forever_casc::Locale::EnUs => 0x2,
        forever_casc::Locale::EsEs => 0x80,
    };
    let build_info_tags = json_quote(&store.build().tags)?;
    let mut report = String::new();
    report
        .try_reserve_exact(MAX_REPORT_BYTES)
        .map_err(|_| Error::from("product_census_limit"))?;
    append(
        &mut report,
        &format!(
            "{{\"schema\":\"{PROFILE}\",\"catalogue\":\"{}\",\"build\":{},\"locale\":\"{}\",\"locale_mask\":{},\"build_info_tags\":{build_info_tags},\"declaration_count\":{declaration_count},\"keyless_availability_census\":true,\"partial_result_expected_when_keys_unavailable\":true,\"complete_extraction_claim\":false,\"limits\":{{\"max_per_candidate_span_bytes\":{},\"max_per_declaration_span_bytes\":{},\"max_total_candidate_span_bytes\":{MAX_CANDIDATE_SPAN_BYTES}}},\"aggregate\":{{",
            catalog::WOWDBDEFS_70450_CATALOG_ID,
            plan.build,
            plan.locale.name(),
            locale_mask,
            probe::Limits::default().max_record_bytes,
            MAX_PER_DECLARATION_BYTES,
        ),
    )?;
    for (index, state) in State::ALL.iter().enumerate() {
        if index != 0 {
            append(&mut report, ",")?;
        }
        append(
            &mut report,
            &format!("\"{}\":{}", state.name(), counts[state.index()]),
        )?;
    }
    append(&mut report, "},\"declarations\":[")?;
    for (index, entry) in entries.iter().enumerate() {
        if index != 0 {
            append(&mut report, ",")?;
        }
        append(&mut report, entry)?;
    }
    append(&mut report, "]}\n")?;

    atomic_write_with(client, &output, |file| {
        file.write_all(report.as_bytes())?;
        file.sync_all()
    })?;
    Ok(Summary { declaration_count })
}

struct TemporaryOutput {
    path: PathBuf,
    cleanup: bool,
}
impl Drop for TemporaryOutput {
    fn drop(&mut self) {
        if self.cleanup {
            let _ = fs::remove_file(&self.path);
        }
    }
}

fn atomic_write_with(
    client: &Path,
    requested: &Path,
    write: impl FnOnce(&mut fs::File) -> std::io::Result<()>,
) -> Result<(), Error> {
    let target = super::probe_encoding::output_path(client, requested)?;
    let parent = target
        .parent()
        .ok_or_else(|| Error::from("product_diagnostic_path"))?
        .to_path_buf();
    let mut temp = None;
    for _ in 0..32 {
        let sequence = NEXT_TEMP.fetch_add(1, Ordering::Relaxed);
        let path = parent.join(format!(
            ".forever-db2-census-{}-{sequence}.tmp",
            std::process::id()
        ));
        match fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .mode(0o600)
            .open(&path)
        {
            Ok(file) => {
                temp = Some((path, file));
                break;
            }
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => continue,
            Err(_) => return Err("product_census_output".into()),
        }
    }
    let Some((temp_path, mut file)) = temp else {
        return Err("product_census_output".into());
    };
    let mut cleanup = TemporaryOutput {
        path: temp_path,
        cleanup: true,
    };
    let write_result = write(&mut file);
    drop(file);
    write_result.map_err(|_| Error::from("product_census_output"))?;

    // Recheck both path guards at the publication boundary. Rename is within
    // this already guarded directory, so readers see either no report or all
    // of the fully written report, never a prefix.
    let target = super::probe_encoding::output_path(client, &target)?;
    fs::rename(&cleanup.path, &target).map_err(|_| Error::from("product_census_output"))?;
    cleanup.cleanup = false;
    fs::File::open(parent)
        .and_then(|directory| directory.sync_all())
        .map_err(|_| Error::from("product_census_output"))?;
    Ok(())
}

fn classify(file: &FileTrace) -> (State, &'static str) {
    // BuildReference rows are not included in encoding_row_count, but their
    // EKeys are also returned by Central::encoded_keys and tried by the real
    // read_selected_ckey path. A successful complete decode through one is
    // therefore fully readable even though it was not an ENCODING row.
    let encodings = file
        .route_rows
        .iter()
        .flat_map(|route| route.encodings.iter());
    if encodings
        .clone()
        .any(|encoding| encoding.read_result == "decoded")
    {
        return (
            State::FullyReadable,
            "complete_keyless_decode_and_ckey_check",
        );
    }
    let classes = file
        .route_rows
        .iter()
        .flat_map(|route| route.encodings.iter())
        .flat_map(|encoding| {
            std::iter::once(encoding.rejection_class).chain(
                encoding
                    .candidates
                    .iter()
                    .map(|candidate| candidate.rejection_class),
            )
        })
        .flatten();
    if classes
        .clone()
        .any(|class| matches!(class, "unsupported" | "corrupt" | "unknown_decoded_size"))
    {
        return (
            State::UnsupportedOrUnknown,
            "candidate_unsupported_or_unverified",
        );
    }
    if classes.clone().any(|class| class == "encrypted") {
        return (
            State::EncryptedNoAvailableKey,
            "encrypted_candidate_no_key_available",
        );
    }
    if file.selected_root_ordinal.is_none() {
        return (
            State::FileUnavailable,
            "no_central_present_eligible_root_row",
        );
    }
    if file.encoding_row_count == 0 {
        return (State::FileUnavailable, "no_encoding_rows_for_selected_ckey");
    }
    if !file
        .route_rows
        .iter()
        .flat_map(|route| route.encodings.iter())
        .any(|encoding| encoding.index_prefix_match)
    {
        return (State::FileUnavailable, "no_local_index_prefix_match");
    }
    if classes.clone().any(|class| class == "incomplete") {
        return (State::FileUnavailable, "local_archive_span_incomplete");
    }
    (State::FileUnavailable, "no_complete_local_encoding")
}

fn classify_error(code: &'static str) -> (State, &'static str) {
    if code == "file_unavailable" {
        (State::FileUnavailable, code)
    } else {
        (State::UnsupportedOrUnknown, code)
    }
}

fn json_quote(value: &str) -> Result<String, Error> {
    if value.len() > MAX_REPORT_BYTES / 4 {
        return Err("product_census_limit".into());
    }
    let mut output = String::new();
    output
        .try_reserve(value.len().saturating_add(2))
        .map_err(|_| Error::from("product_census_limit"))?;
    output.push('"');
    for ch in value.chars() {
        match ch {
            '"' => output.push_str("\\\""),
            '\\' => output.push_str("\\\\"),
            '\n' => output.push_str("\\n"),
            '\r' => output.push_str("\\r"),
            '\t' => output.push_str("\\t"),
            ch if ch < ' ' => output.push_str(&format!("\\u{:04x}", ch as u32)),
            ch => output.push(ch),
        }
        if output.len() > MAX_REPORT_BYTES / 2 {
            return Err("product_census_limit".into());
        }
    }
    output.push('"');
    Ok(output)
}

fn append(output: &mut String, value: &str) -> Result<(), Error> {
    let end = output
        .len()
        .checked_add(value.len())
        .ok_or_else(|| Error::from("product_census_limit"))?;
    if end > MAX_REPORT_BYTES {
        return Err("product_census_limit".into());
    }
    output.push_str(value);
    Ok(())
}

#[cfg(test)]
#[path = "census_db2/tests.rs"]
mod tests;
