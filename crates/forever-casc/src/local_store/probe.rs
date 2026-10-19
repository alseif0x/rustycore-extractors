// SPDX-License-Identifier: GPL-3.0-or-later
//! Bounded redacted ROOT/ENCODING/index trace for one caller-selected FDID set.
//! It never returns CKey/EKey bytes, archive locations, encoded spans or decoded
//! payloads. Candidate reads use the existing keyless complete decoder and only
//! retain a status, rejection class and static BLTE code.

use super::{Budget, Error as StoreError, LocalStore, add, array_bytes, io, owned, routes};
use crate::Locale;
use std::{cell::Cell, fmt};

pub const PROFILE: &str = "forever-local-encoding-probe-v1";
pub const MAX_FILE_DATA_IDS: usize = 64;
pub const MAX_TRACE_BYTES: usize = 4 * 1024 * 1024;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Error {
    KeysPresent,
    Limit,
    Store(StoreError),
}
impl Error {
    pub const fn code(self) -> &'static str {
        match self {
            Self::KeysPresent => "probe_encoding_keys_forbidden",
            Self::Limit => "probe_encoding_limit",
            Self::Store(error) => error.code(),
        }
    }
}
impl From<StoreError> for Error {
    fn from(error: StoreError) -> Self {
        Self::Store(error)
    }
}
impl std::fmt::Display for Error {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(self.code())
    }
}
impl std::error::Error for Error {}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Limits {
    pub routes: routes::Limits,
    pub max_route_rows_per_file: usize,
    pub max_route_rows_total: usize,
    pub max_record_bytes: u64,
    pub max_total_record_bytes: u64,
    pub max_decoded_bytes: u64,
    pub max_frames: u32,
}
impl Default for Limits {
    fn default() -> Self {
        Self {
            routes: routes::Limits::default(),
            max_route_rows_per_file: 64,
            max_route_rows_total: 256,
            max_record_bytes: 16 * 1024 * 1024,
            max_total_record_bytes: 256 * 1024 * 1024,
            max_decoded_bytes: 64 * 1024 * 1024,
            max_frames: 65_536,
        }
    }
}

pub struct RootRow {
    pub ordinal: u32,
    pub block_index: u32,
    pub locale_flags: u32,
    pub effective_content_flags: u32,
    pub audio_locale_bit: bool,
    pub eligible_for_locale: bool,
    pub ckey_central_present: bool,
    pub selected: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RouteKind {
    Encoding,
    BuildReference,
}
impl RouteKind {
    const fn name(self) -> &'static str {
        match self {
            Self::Encoding => "encoding",
            Self::BuildReference => "build_reference",
        }
    }
}

pub struct CandidateRead {
    pub span_ordinal: u32,
    pub read_result: &'static str,
    pub rejection_class: Option<&'static str>,
    pub blte_code: Option<&'static str>,
}
pub struct Encoding {
    pub ordinal: u32,
    pub content_size: Option<u32>,
    pub span_count: u32,
    pub index_prefix_match: bool,
    pub read_result: &'static str,
    pub rejection_class: Option<&'static str>,
    pub blte_code: Option<&'static str>,
    pub candidates: Vec<CandidateRead>,
}
pub struct RouteRow {
    pub kind: RouteKind,
    pub ordinal: u32,
    pub content_size: Option<u32>,
    pub encoding_count: u32,
    pub encodings: Vec<Encoding>,
}
pub struct FileTrace {
    pub file_data_id: u32,
    pub root_rows_considered: u32,
    pub root_rows: Vec<RootRow>,
    pub selected_root_ordinal: Option<u32>,
    pub encoding_row_count: u32,
    /// Includes separately labelled configuration build-reference rows because
    /// those can also contribute routes to the exact LocalStore read path.
    pub route_rows: Vec<RouteRow>,
}
pub struct Report {
    pub build: u32,
    pub locale: Locale,
    pub locale_mask: u32,
    pub build_info_tags: String,
    pub files: Vec<FileTrace>,
}

impl Report {
    /// Deterministic JSON with an intentionally closed, redacted field set.
    pub fn render(&self) -> Result<String, Error> {
        let mut output = JsonOutput::new()?;
        write_bounded(
            &mut output,
            format_args!(
                "{{\"schema\":\"{PROFILE}\",\"build\":{},\"locale\":\"{}\",\"files\":[",
                self.build,
                self.locale.name(),
            ),
        )?;
        for (file_index, file) in self.files.iter().enumerate() {
            if file_index != 0 {
                output.push(',')?;
            }
            write_bounded(
                &mut output,
                format_args!(
                    "{{\"file_data_id\":{},\"root_rows_considered\":{},\"selected_root_ordinal\":",
                    file.file_data_id, file.root_rows_considered
                ),
            )?;
            json_optional_number(&mut output, file.selected_root_ordinal.map(u64::from))?;
            write_bounded(
                &mut output,
                format_args!(
                    ",\"encoding_row_count\":{},\"root_rows\":[",
                    file.encoding_row_count
                ),
            )?;
            for (row_index, row) in file.root_rows.iter().enumerate() {
                if row_index != 0 {
                    output.push(',')?;
                }
                write_bounded(
                    &mut output,
                    format_args!(
                        "{{\"ordinal\":{},\"block_index\":{},\"locale_flags\":{},\"effective_content_flags\":{},\"audio_locale_bit\":{},\"eligible_for_locale\":{},\"ckey_central_present\":{},\"selected\":{}}}",
                        row.ordinal,
                        row.block_index,
                        row.locale_flags,
                        row.effective_content_flags,
                        row.audio_locale_bit,
                        row.eligible_for_locale,
                        row.ckey_central_present,
                        row.selected,
                    ),
                )?;
            }
            output.push_str("],\"route_rows\":[")?;
            for (route_index, route) in file.route_rows.iter().enumerate() {
                if route_index != 0 {
                    output.push(',')?;
                }
                write_bounded(
                    &mut output,
                    format_args!(
                        "{{\"kind\":\"{}\",\"ordinal\":{},\"content_size\":",
                        route.kind.name(),
                        route.ordinal
                    ),
                )?;
                json_optional_number(&mut output, route.content_size.map(u64::from))?;
                write_bounded(
                    &mut output,
                    format_args!(
                        ",\"encoding_count\":{},\"encodings\":[",
                        route.encoding_count
                    ),
                )?;
                for (encoding_index, encoding) in route.encodings.iter().enumerate() {
                    if encoding_index != 0 {
                        output.push(',')?;
                    }
                    write_bounded(
                        &mut output,
                        format_args!("{{\"ordinal\":{},\"content_size\":", encoding.ordinal),
                    )?;
                    json_optional_number(&mut output, encoding.content_size.map(u64::from))?;
                    write_bounded(
                        &mut output,
                        format_args!(
                            ",\"span_count\":{},\"index_prefix_match\":{},\"read_result\":\"{}\",\"rejection_class\":",
                            encoding.span_count, encoding.index_prefix_match, encoding.read_result
                        ),
                    )?;
                    json_optional_string(&mut output, encoding.rejection_class)?;
                    output.push_str(",\"blte_code\":")?;
                    json_optional_string(&mut output, encoding.blte_code)?;
                    output.push_str(",\"candidates\":[")?;
                    for (candidate_index, candidate) in encoding.candidates.iter().enumerate() {
                        if candidate_index != 0 {
                            output.push(',')?;
                        }
                        write_bounded(
                            &mut output,
                            format_args!(
                                "{{\"span_ordinal\":{},\"read_result\":\"{}\",\"rejection_class\":",
                                candidate.span_ordinal, candidate.read_result
                            ),
                        )?;
                        json_optional_string(&mut output, candidate.rejection_class)?;
                        output.push_str(",\"blte_code\":")?;
                        json_optional_string(&mut output, candidate.blte_code)?;
                        output.push('}')?;
                    }
                    output.push_str("]}")?;
                }
                output.push_str("]}")?;
            }
            output.push_str("]}")?;
        }
        write_bounded(
            &mut output,
            format_args!(
                "],\"locale_mask\":{},\"build_info_tags\":",
                self.locale_mask
            ),
        )?;
        json_string(&mut output, &self.build_info_tags)?;
        output.push_str("}\n")?;
        Ok(output.into_string())
    }
}

struct JsonOutput(String);
impl JsonOutput {
    fn new() -> Result<Self, Error> {
        let mut output = String::new();
        output
            .try_reserve_exact(MAX_TRACE_BYTES)
            .map_err(|_| Error::Limit)?;
        Ok(Self(output))
    }
    fn push_str(&mut self, value: &str) -> Result<(), Error> {
        let end = self.0.len().checked_add(value.len()).ok_or(Error::Limit)?;
        if end > MAX_TRACE_BYTES {
            return Err(Error::Limit);
        }
        self.0.push_str(value);
        Ok(())
    }
    fn push(&mut self, value: char) -> Result<(), Error> {
        let end = self
            .0
            .len()
            .checked_add(value.len_utf8())
            .ok_or(Error::Limit)?;
        if end > MAX_TRACE_BYTES {
            return Err(Error::Limit);
        }
        self.0.push(value);
        Ok(())
    }
    fn into_string(self) -> String {
        self.0
    }
}
impl fmt::Write for JsonOutput {
    fn write_str(&mut self, value: &str) -> fmt::Result {
        self.push_str(value).map_err(|_| fmt::Error)
    }
}
fn write_bounded(output: &mut JsonOutput, args: fmt::Arguments<'_>) -> Result<(), Error> {
    fmt::write(output, args).map_err(|_| Error::Limit)
}

fn json_optional_number(output: &mut JsonOutput, value: Option<u64>) -> Result<(), Error> {
    match value {
        Some(value) => write_bounded(output, format_args!("{value}")),
        None => output.push_str("null"),
    }
}
fn json_optional_string(output: &mut JsonOutput, value: Option<&str>) -> Result<(), Error> {
    if let Some(value) = value {
        json_string(output, value)
    } else {
        output.push_str("null")
    }
}
fn json_string(output: &mut JsonOutput, value: &str) -> Result<(), Error> {
    output.push('"')?;
    for ch in value.chars() {
        match ch {
            '"' => output.push_str("\\\"")?,
            '\\' => output.push_str("\\\\")?,
            '\n' => output.push_str("\\n")?,
            '\r' => output.push_str("\\r")?,
            '\t' => output.push_str("\\t")?,
            ch if ch < ' ' => write_bounded(output, format_args!("\\u{:04x}", ch as u32))?,
            ch => output.push(ch)?,
        }
    }
    output.push('"')?;
    Ok(())
}

impl LocalStore {
    /// Redacted per-FDID ROOT and route trace. Payload bytes and key material are
    /// never returned; every exact candidate is decoded only inside LocalStore.
    pub fn probe_file_data_ids(
        &self,
        file_data_ids: &[u32],
        limits: Limits,
    ) -> Result<Report, Error> {
        if self.keys.is_some() {
            return Err(Error::KeysPresent);
        }
        if file_data_ids.is_empty()
            || file_data_ids.len() > MAX_FILE_DATA_IDS
            || file_data_ids.windows(2).any(|pair| pair[0] >= pair[1])
        {
            return Err(Error::Limit);
        }
        let defaults = Limits::default();
        if limits.routes.max_root_rows_per_input > defaults.routes.max_root_rows_per_input
            || limits.routes.max_candidates_per_input > defaults.routes.max_candidates_per_input
            || limits.routes.max_candidates_per_input > self.limits.max_candidate_attempts
            || limits.routes.max_candidates_total > defaults.routes.max_candidates_total
            || limits.max_route_rows_per_file > defaults.max_route_rows_per_file
            || limits.max_route_rows_total > defaults.max_route_rows_total
            || limits.max_record_bytes > defaults.max_record_bytes
            || limits.max_total_record_bytes > defaults.max_total_record_bytes
            || limits.max_decoded_bytes > defaults.max_decoded_bytes
            || limits.max_frames > defaults.max_frames
        {
            return Err(Error::Limit);
        }
        io::check_guards(&self.guards)?;
        let result = self.probe_inner(file_data_ids, limits);
        io::check_guards(&self.guards)?;
        result
    }

    fn probe_inner(&self, ids: &[u32], limits: Limits) -> Result<Report, Error> {
        let mut budget = Budget {
            limits: self.limits,
            resident: self.stats.resident_bytes,
            scratch: Cell::new(0),
        };
        let scratch = self
            .limits
            .max_path_bytes
            .checked_mul(2)
            .ok_or(StoreError::Limit)?;
        budget.phase(scratch)?;
        let mut files = Vec::new();
        files
            .try_reserve_exact(ids.len())
            .map_err(|_| StoreError::Limit)?;
        budget.retain(array_bytes::<FileTrace>(files.capacity())?)?;
        let mut candidate_count = 0usize;
        let mut candidate_bytes = 0u64;
        let mut route_row_count = 0usize;
        for &id in ids {
            let root_count = self
                .root
                .matching_with_ordinals(id)
                .take(limits.routes.max_root_rows_per_input + 1)
                .count();
            if root_count > limits.routes.max_root_rows_per_input {
                return Err(Error::Limit);
            }
            let mut root_rows = Vec::new();
            root_rows
                .try_reserve_exact(root_count)
                .map_err(|_| StoreError::Limit)?;
            budget.retain(array_bytes::<RootRow>(root_rows.capacity())?)?;
            let selected = self
                .root
                .resolve_with_ordinal(id, self.locale, |key| self.central.contains(key));
            for (ordinal, row) in self.root.matching_with_ordinals(id) {
                let block = self
                    .root
                    .blocks()
                    .get(row.block_index)
                    .ok_or(StoreError::Integrity)?;
                let flags = block.content_flags.effective();
                let is_eligible = self.root.eligible_row(row, self.locale);
                root_rows.push(RootRow {
                    ordinal: u32::try_from(ordinal).map_err(|_| Error::Limit)?,
                    block_index: u32::try_from(row.block_index).map_err(|_| Error::Limit)?,
                    locale_flags: block.locale_flags,
                    effective_content_flags: flags,
                    audio_locale_bit: flags >> 31 != 0,
                    eligible_for_locale: is_eligible,
                    ckey_central_present: self.central.contains(row.content_key),
                    selected: selected
                        .is_some_and(|(selected_ordinal, _)| selected_ordinal == ordinal),
                });
            }
            root_rows.sort_unstable_by_key(|row| row.ordinal);
            let selected_root_ordinal = selected
                .map(|(ordinal, _)| u32::try_from(ordinal).map_err(|_| Error::Limit))
                .transpose()?;
            let mut route_rows = Vec::new();
            let mut encoding_row_ordinals = 0u32;
            let mut build_reference_ordinals = 0u32;
            let mut encoding_row_count = 0u32;
            let mut file_candidate_count = 0usize;
            if let Some((_, selected_root)) = selected {
                let ckey = selected_root.content_key;
                let effective_size = self.central.content_size(ckey);
                let selected_route_count = self.central.probe_routes(ckey).count();
                route_row_count = add(route_row_count, selected_route_count)?;
                if route_row_count > limits.max_route_rows_total
                    || selected_route_count > limits.max_route_rows_per_file
                {
                    return Err(Error::Limit);
                }
                route_rows
                    .try_reserve_exact(selected_route_count)
                    .map_err(|_| StoreError::Limit)?;
                budget.retain(array_bytes::<RouteRow>(route_rows.capacity())?)?;
                for route in self.central.probe_routes(ckey) {
                    let (kind, route_ordinal) = if route.build_reference {
                        let ordinal = build_reference_ordinals;
                        build_reference_ordinals = build_reference_ordinals
                            .checked_add(1)
                            .ok_or(Error::Limit)?;
                        (RouteKind::BuildReference, ordinal)
                    } else {
                        encoding_row_count =
                            encoding_row_count.checked_add(1).ok_or(Error::Limit)?;
                        let ordinal = encoding_row_ordinals;
                        encoding_row_ordinals =
                            encoding_row_ordinals.checked_add(1).ok_or(Error::Limit)?;
                        (RouteKind::Encoding, ordinal)
                    };
                    let encoding_count = route.encoded_keys.len();
                    if encoding_count > limits.routes.max_candidates_per_input {
                        return Err(Error::Limit);
                    }
                    let mut encodings = Vec::new();
                    encodings
                        .try_reserve_exact(encoding_count)
                        .map_err(|_| StoreError::Limit)?;
                    budget.retain(array_bytes::<Encoding>(encodings.capacity())?)?;
                    for (encoding_ordinal, ekey) in route.encoded_keys.iter().enumerate() {
                        let range = owned::equal_range(&self.index_order, |index| {
                            self.indexes[index].key_prefix.as_slice().cmp(&ekey[..9])
                        });
                        let span_count = range.len();
                        let attempts = span_count.max(1);
                        file_candidate_count = add(file_candidate_count, attempts)?;
                        candidate_count = add(candidate_count, attempts)?;
                        if file_candidate_count > limits.routes.max_candidates_per_input
                            || candidate_count > limits.routes.max_candidates_total
                            || attempts > limits.routes.max_candidates_per_input
                        {
                            return Err(Error::Limit);
                        }
                        let mut candidates = Vec::new();
                        candidates
                            .try_reserve_exact(span_count)
                            .map_err(|_| StoreError::Limit)?;
                        budget.retain(array_bytes::<CandidateRead>(candidates.capacity())?)?;
                        let mut summary_class: Option<&'static str> = None;
                        let mut summary_blte: Option<&'static str> = None;
                        let mut any_decoded = false;
                        if span_count == 0 {
                            summary_class = Some("missing");
                        } else if effective_size.is_none() {
                            summary_class = Some("unknown_decoded_size");
                        } else {
                            for (span_ordinal, &index) in self.index_order[range].iter().enumerate()
                            {
                                let entry = &self.indexes[index as usize];
                                if u64::from(entry.encoded_size) > limits.max_record_bytes {
                                    return Err(Error::Limit);
                                }
                                candidate_bytes = candidate_bytes
                                    .checked_add(u64::from(entry.encoded_size))
                                    .ok_or(Error::Limit)?;
                                if candidate_bytes > limits.max_total_record_bytes {
                                    return Err(Error::Limit);
                                }
                                let reader = io::Reader {
                                    dir: &self.data_dir,
                                    indexes: &self.indexes,
                                    order: &self.index_order,
                                    limits: super::Limits {
                                        max_archive_span_bytes: self
                                            .limits
                                            .max_archive_span_bytes
                                            .min(limits.max_record_bytes),
                                        max_decoded_file_bytes: self
                                            .limits
                                            .max_decoded_file_bytes
                                            .min(limits.max_decoded_bytes),
                                        max_blte_frames: self
                                            .limits
                                            .max_blte_frames
                                            .min(limits.max_frames),
                                        ..self.limits
                                    },
                                    budget: &budget,
                                    keys: None,
                                };
                                let read = reader.probe_candidate(
                                    entry,
                                    *ekey,
                                    ckey,
                                    effective_size,
                                    limits
                                        .max_decoded_bytes
                                        .min(self.limits.max_decoded_file_bytes),
                                )?;
                                match read {
                                    io::ProbeRead::Decoded => {
                                        any_decoded = true;
                                        candidates.push(CandidateRead {
                                            span_ordinal: span_ordinal as u32,
                                            read_result: "decoded",
                                            rejection_class: None,
                                            blte_code: None,
                                        });
                                    }
                                    io::ProbeRead::Rejected { class, blte_code } => {
                                        if summary_class.is_none() {
                                            summary_class = Some(class);
                                        } else if summary_class != Some(class) {
                                            summary_class = Some("mixed");
                                        }
                                        if summary_blte.is_none() {
                                            summary_blte = blte_code;
                                        }
                                        candidates.push(CandidateRead {
                                            span_ordinal: span_ordinal as u32,
                                            read_result: "rejected",
                                            rejection_class: Some(class),
                                            blte_code,
                                        });
                                    }
                                }
                            }
                        }
                        let (read_result, rejection_class) = if any_decoded {
                            ("decoded", None)
                        } else if span_count == 0 {
                            ("rejected", Some("missing"))
                        } else if effective_size.is_none() {
                            ("rejected", Some("unknown_decoded_size"))
                        } else {
                            ("rejected", summary_class)
                        };
                        encodings.push(Encoding {
                            ordinal: u32::try_from(encoding_ordinal).map_err(|_| Error::Limit)?,
                            content_size: route.content_size,
                            span_count: u32::try_from(span_count).map_err(|_| Error::Limit)?,
                            index_prefix_match: span_count != 0,
                            read_result,
                            rejection_class,
                            blte_code: summary_blte,
                            candidates,
                        });
                    }
                    route_rows.push(RouteRow {
                        kind,
                        ordinal: route_ordinal,
                        content_size: route.content_size,
                        encoding_count: u32::try_from(route.encoded_keys.len())
                            .map_err(|_| Error::Limit)?,
                        encodings,
                    });
                }
            }
            files.push(FileTrace {
                file_data_id: id,
                root_rows_considered: u32::try_from(root_count).map_err(|_| Error::Limit)?,
                root_rows,
                selected_root_ordinal,
                encoding_row_count,
                route_rows,
            });
        }
        Ok(Report {
            build: self.build.build,
            locale: self.locale,
            locale_mask: locale_mask(self.locale),
            build_info_tags: self.build.tags.clone(),
            files,
        })
    }
}

fn locale_mask(locale: Locale) -> u32 {
    match locale {
        Locale::EnUs => 0x2,
        Locale::EsEs => 0x80,
    }
}

#[cfg(all(test, target_os = "linux"))]
#[path = "probe_tests.rs"]
mod tests;
