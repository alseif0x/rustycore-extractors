// SPDX-License-Identifier: GPL-3.0-or-later
// Authored 2026-10-11: closed, key-free complete local-span framing observations.
//! Reads only the three required inputs, without decoding or exposing raw routes.
//! Complete framing evidence does not authenticate full EKeys, CKeys or plaintext.

use super::{Budget, LocalStore, add, array_bytes, io, owned, reserve, routes};
use crate::blte;
use routes::{RequiredInput, RouteState, SpanState};
use std::cell::Cell;

pub const PROFILE: &str = "forever-local-required-framing-v1";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Limits {
    pub routes: routes::Limits,
    /// Complete local record, including the ignored 30-byte prelude.
    pub max_record_bytes: u64,
    /// Conservative sum of every indexed candidate, including unreadable ones.
    pub max_total_record_bytes: u64,
    pub max_declared_content_bytes: u64,
    pub max_frames: u32,
    /// Sum of retained output candidate capacities; excludes caller serialization.
    pub max_report_bytes: usize,
}
impl Default for Limits {
    fn default() -> Self {
        Self {
            routes: routes::Limits::default(),
            max_record_bytes: 128 * 1024,
            max_total_record_bytes: 256 * 1024,
            max_declared_content_bytes: 512 * 1024,
            max_frames: 4096,
            max_report_bytes: 128 * 1024,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Inspection {
    /// Missing/unsupported extent or unknown expected decoded size: not read.
    NotRead,
    /// Complete read, but no framing evidence escaped the rejected inspection.
    Rejected(&'static str),
    Complete(blte::framing::Evidence),
}
// No Debug or raw identity accessors on route-bearing reports.
pub struct Candidate {
    pub encoding_ordinal: u32,
    pub index_match_ordinal: Option<u32>,
    pub encoded_record_bytes: Option<u32>,
    pub observed_archive_bytes: Option<u64>,
    pub span: SpanState,
    pub inspection: Inspection,
}
pub struct Diagnostic {
    pub input: RequiredInput,
    pub root_rows: u32,
    pub eligible_root_rows: u32,
    pub selected_root_ordinal: Option<u32>,
    pub expected_decoded_bytes: Option<u32>,
    pub route: RouteState,
    pub candidates: Vec<Candidate>,
}
pub type Error = routes::Error;

// These never escape this module, including through Debug or error messages.
struct Choice {
    index: Option<u32>,
    ekey: [u8; 16],
    encoding_ordinal: u32,
    index_match_ordinal: Option<u32>,
}

impl LocalStore {
    /// Inspect all candidates in the closed selection, preserving order.
    /// No acquisition fallback, decrypt, inflate, payload output or key lookup.
    /// All caps and conservative total bytes are admitted before any payload IO.
    /// Final store guards run on both successful and failed operations.
    pub fn diagnose_required_framing(&self, limits: Limits) -> Result<[Diagnostic; 3], Error> {
        routes::guarded(self, || self.required_framing_inner(limits))
    }

    fn required_framing_inner(&self, limits: Limits) -> Result<[Diagnostic; 3], Error> {
        if self.keys.is_some() {
            return Err(Error::KeysPresent);
        }
        let caps = Limits::default();
        if limits.routes.max_root_rows_per_input > caps.routes.max_root_rows_per_input
            || limits.routes.max_candidates_per_input > caps.routes.max_candidates_per_input
            || limits.routes.max_candidates_per_input > self.limits.max_candidate_attempts
            || limits.routes.max_candidates_total > caps.routes.max_candidates_total
            || limits.max_record_bytes > caps.max_record_bytes
            || limits.max_total_record_bytes > caps.max_total_record_bytes
            || limits.max_declared_content_bytes > caps.max_declared_content_bytes
            || limits.max_frames > caps.max_frames
            || limits.max_report_bytes > caps.max_report_bytes
        {
            return Err(super::Error::Limit.into());
        }
        let mut budget = Budget {
            limits: self.limits,
            resident: self.stats.resident_bytes,
            scratch: Cell::new(0),
        };
        let scratch = self
            .limits
            .max_path_bytes
            .checked_mul(2)
            .ok_or(super::Error::Limit)?;
        #[cfg(windows)]
        let scratch = add(scratch, io::windows_lease_reservation())?;
        budget.phase(scratch)?;
        let mut output = [
            RequiredInput::Map,
            RequiredInput::SkillLineAbility,
            RequiredInput::GameObjectDisplayInfo,
        ]
        .map(|input| Diagnostic {
            input,
            root_rows: 0,
            eligible_root_rows: 0,
            selected_root_ordinal: None,
            expected_decoded_bytes: None,
            route: RouteState::RootAbsent,
            candidates: Vec::new(),
        });
        let mut plans: [Vec<Choice>; 3] = std::array::from_fn(|_| Vec::new());
        let mut total_candidates = 0usize;
        let mut total_bytes = 0u64;
        let mut report_bytes = array_bytes::<Diagnostic>(3)?;
        for (diagnostic, plan) in output.iter_mut().zip(&mut plans) {
            budget.retain(array_bytes::<Choice>(
                limits.routes.max_candidates_per_input,
            )?)?;
            reserve(plan, limits.routes.max_candidates_per_input)?;
            budget.retain(array_bytes::<Choice>(
                plan.capacity() - limits.routes.max_candidates_per_input,
            )?)?;
            for (ordinal, _) in self
                .root
                .matching(diagnostic.input.file_data_id())
                .take(limits.routes.max_root_rows_per_input + 1)
                .enumerate()
            {
                if ordinal >= limits.routes.max_root_rows_per_input {
                    return Err(super::Error::Limit.into());
                }
                diagnostic.root_rows += 1;
            }
            let mut selected = None;
            for (ordinal, row) in self
                .root
                .eligible(diagnostic.input.file_data_id(), self.locale)
                .enumerate()
            {
                if ordinal >= limits.routes.max_root_rows_per_input {
                    return Err(super::Error::Limit.into());
                }
                diagnostic.eligible_root_rows += 1;
                if selected.is_none() && self.central.contains(row.content_key) {
                    selected = Some(row.content_key);
                    diagnostic.selected_root_ordinal = Some(ordinal as u32);
                }
            }
            if diagnostic.eligible_root_rows == 0 {
                if diagnostic.root_rows != 0 {
                    diagnostic.route = RouteState::RootIneligible;
                }
                continue;
            }
            let Some(key) = selected else {
                diagnostic.route = RouteState::CentralAbsent;
                continue;
            };
            diagnostic.expected_decoded_bytes = self.central.content_size(key);
            if diagnostic.expected_decoded_bytes.is_some_and(|n| {
                u64::from(n)
                    > limits
                        .max_declared_content_bytes
                        .min(self.limits.max_decoded_file_bytes)
            }) {
                return Err(super::Error::Limit.into());
            }
            diagnostic.route = if diagnostic.expected_decoded_bytes.is_some() {
                RouteState::EncodingAbsent
            } else {
                RouteState::UnknownDecodedSize
            };
            for (encoding_ordinal, ekey) in self.central.encoded_keys(key).enumerate() {
                let range = owned::equal_range(&self.index_order, |index| {
                    self.indexes[index].key_prefix.as_slice().cmp(&ekey[..9])
                });
                let count = range.len().max(1);
                let next = add(plan.len(), count)?;
                total_candidates = add(total_candidates, count)?;
                if next > limits.routes.max_candidates_per_input
                    || total_candidates > limits.routes.max_candidates_total
                {
                    return Err(super::Error::Limit.into());
                }
                if diagnostic.expected_decoded_bytes.is_some() {
                    diagnostic.route = RouteState::Routed;
                }
                if range.is_empty() {
                    plan.push(Choice {
                        index: None,
                        ekey,
                        encoding_ordinal: encoding_ordinal as u32,
                        index_match_ordinal: None,
                    });
                } else {
                    for (ordinal, &index) in self.index_order[range].iter().enumerate() {
                        let entry = &self.indexes[index as usize];
                        let size = u64::from(entry.encoded_size);
                        if size
                            > limits
                                .max_record_bytes
                                .min(self.limits.max_archive_span_bytes)
                        {
                            return Err(super::Error::Limit.into());
                        }
                        total_bytes = total_bytes.checked_add(size).ok_or(super::Error::Limit)?;
                        if total_bytes > limits.max_total_record_bytes {
                            return Err(super::Error::Limit.into());
                        }
                        plan.push(Choice {
                            index: Some(index),
                            ekey,
                            encoding_ordinal: encoding_ordinal as u32,
                            index_match_ordinal: Some(ordinal as u32),
                        });
                    }
                }
            }
            let requested = array_bytes::<Candidate>(plan.len())?;
            report_bytes = add(report_bytes, requested)?;
            if report_bytes > limits.max_report_bytes {
                return Err(super::Error::Limit.into());
            }
            budget.retain(requested)?;
            reserve(&mut diagnostic.candidates, plan.len())?;
            let excess = array_bytes::<Candidate>(diagnostic.candidates.capacity() - plan.len())?;
            report_bytes = add(report_bytes, excess)?;
            if report_bytes > limits.max_report_bytes {
                return Err(super::Error::Limit.into());
            }
            budget.retain(excess)?;
        }
        if report_bytes > limits.max_report_bytes {
            return Err(super::Error::Limit.into());
        }
        // No file payload is touched until every plan and output reservation passes.
        for (diagnostic, plan) in output.iter_mut().zip(&plans) {
            for choice in plan {
                let (encoded_record_bytes, span, observed_archive_bytes, inspection) =
                    if let Some(index) = choice.index {
                        let entry = &self.indexes[index as usize];
                        let (span, observed, inspection) =
                            if let Some(expected) = diagnostic.expected_decoded_bytes {
                                io::framing::inspect_span(io::framing::Request {
                                    directory: &self.data_dir,
                                    entry,
                                    ekey: choice.ekey,
                                    expected: expected.into(),
                                    store_limits: self.limits,
                                    limits,
                                    budget: &budget,
                                    scratch,
                                })?
                            } else {
                                let (span, observed) =
                                    io::observe_span(&self.data_dir, entry, self.limits)?;
                                (span, observed, Inspection::NotRead)
                            };
                        (Some(entry.encoded_size), span, observed, inspection)
                    } else {
                        (None, SpanState::IndexAbsent, None, Inspection::NotRead)
                    };
                diagnostic.candidates.push(Candidate {
                    encoding_ordinal: choice.encoding_ordinal,
                    index_match_ordinal: choice.index_match_ordinal,
                    encoded_record_bytes,
                    observed_archive_bytes,
                    span,
                    inspection,
                });
            }
        }
        Ok(output)
    }
}
