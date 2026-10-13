// SPDX-License-Identifier: GPL-3.0-or-later
// Authored 2026-10-11: original closed, key-free offline route projection.
//! Redacted metadata diagnostics for three explicitly authorized incomplete
//! inputs. This is not acquisition, ciphertext validation or a repair plan.
//! No CKey/EKey, archive path/offset, key identifier or payload is returned.
//! Index matches authenticate only a nine-byte prefix. Even a fitting span
//! leaves its bytes, framing, encryption and complete content uninspected.

use super::{Budget, LocalStore, add, array_bytes, io, owned, reserve};
use std::cell::Cell;

pub const PROFILE: &str = "forever-local-required-routes-v1";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RequiredInput {
    Map,
    SkillLineAbility,
    GameObjectDisplayInfo,
}
impl RequiredInput {
    pub const fn file_data_id(self) -> u32 {
        match self {
            Self::Map => 1_349_477,
            Self::SkillLineAbility => 1_266_278,
            Self::GameObjectDisplayInfo => 1_266_277,
        }
    }
    pub const fn name(self) -> &'static str {
        match self {
            Self::Map => "Map",
            Self::SkillLineAbility => "SkillLineAbility",
            Self::GameObjectDisplayInfo => "GameObjectDisplayInfo",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Limits {
    /// Includes all ROOT rows for the target, including ineligible duplicates.
    pub max_root_rows_per_input: usize,
    /// Every index collision counts; an EKey with no match counts once.
    pub max_candidates_per_input: usize,
    pub max_candidates_total: usize,
}
impl Default for Limits {
    fn default() -> Self {
        Self {
            max_root_rows_per_input: 64,
            max_candidates_per_input: 64,
            max_candidates_total: 192,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RouteState {
    RootAbsent,
    RootIneligible,
    CentralAbsent,
    UnknownDecodedSize,
    EncodingAbsent,
    Routed,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SpanState {
    IndexAbsent,
    ArchiveAbsent,
    ArchiveRejected,
    UnsupportedArchive,
    InvalidExtent,
    IncompleteSpan,
    FitsObservedLength,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Inspection {
    NotInspected,
}

// Deliberately no Debug, raw identity accessor, or hidden correlation fields.
pub struct Candidate {
    pub encoding_ordinal: u32,
    pub index_match_ordinal: Option<u32>,
    pub encoded_record_bytes: Option<u32>,
    pub observed_archive_bytes: Option<u64>,
    pub span: SpanState,
    pub payload: Inspection,
}
pub struct Diagnostic {
    pub input: RequiredInput,
    pub root_rows: u32,
    pub eligible_root_rows: u32,
    /// Ordinal in the eligible ROOT selection order, not the source file.
    pub selected_root_ordinal: Option<u32>,
    pub expected_decoded_bytes: Option<u32>,
    pub route: RouteState,
    pub candidates: Vec<Candidate>,
    pub ciphertext: Inspection,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Error {
    KeysPresent,
    Store(super::Error),
}
impl Error {
    pub const fn code(self) -> &'static str {
        match self {
            Self::KeysPresent => "route_diagnostic_requires_key_free_store",
            Self::Store(error) => error.code(),
        }
    }
}
impl From<super::Error> for Error {
    fn from(error: super::Error) -> Self {
        Self::Store(error)
    }
}
impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.code())
    }
}
impl std::error::Error for Error {}

impl LocalStore {
    /// Project only Map, SkillLineAbility and GameObjectDisplayInfo, in that
    /// order, from already admitted metadata. No archive byte is read. Existing
    /// store guards (including the exact admitted build-info bytes) are checked
    /// before and after the operation, including failed operations. Each archive
    /// metadata observation holds and verifies one readonly lease at a time.
    /// A failure returns no prefix report. Limits only reduce the closed caps
    /// and the store's acquisition attempt bound; the archive-span limit applies.
    pub fn diagnose_required_routes(&self, limits: Limits) -> Result<[Diagnostic; 3], Error> {
        guarded(self, || self.diagnose_routes_inner(limits))
    }

    fn diagnose_routes_inner(&self, limits: Limits) -> Result<[Diagnostic; 3], Error> {
        if self.keys.is_some() {
            return Err(Error::KeysPresent);
        }
        let caps = Limits::default();
        if limits.max_root_rows_per_input > caps.max_root_rows_per_input
            || limits.max_candidates_per_input > caps.max_candidates_per_input
            || limits.max_candidates_per_input > self.limits.max_candidate_attempts
            || limits.max_candidates_total > caps.max_candidates_total
        {
            return Err(super::Error::Limit.into());
        }
        let mut budget = Budget {
            limits: self.limits,
            resident: self.stats.resident_bytes,
            scratch: Cell::new(0),
        };
        // One bounded archive path; Windows additionally opens one ancestor
        // lease. Requested vector capacities and observed excess are charged.
        let scratch = self
            .limits
            .max_path_bytes
            .checked_mul(2)
            .ok_or(super::Error::Limit)?;
        #[cfg(windows)]
        let scratch = add(scratch, io::windows_lease_reservation())?;
        budget.phase(scratch)?;
        let mut diagnostics = [
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
            ciphertext: Inspection::NotInspected,
        });
        let mut total = 0usize;
        for diagnostic in &mut diagnostics {
            budget.retain(array_bytes::<Candidate>(limits.max_candidates_per_input)?)?;
            reserve(&mut diagnostic.candidates, limits.max_candidates_per_input)?;
            budget.retain(array_bytes::<Candidate>(
                diagnostic.candidates.capacity() - limits.max_candidates_per_input,
            )?)?;
            for (ordinal, _) in self
                .root
                .matching(diagnostic.input.file_data_id())
                .take(limits.max_root_rows_per_input + 1)
                .enumerate()
            {
                if ordinal >= limits.max_root_rows_per_input {
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
                if ordinal >= limits.max_root_rows_per_input {
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
                let next = add(diagnostic.candidates.len(), count)?;
                total = add(total, count)?;
                if next > limits.max_candidates_per_input || total > limits.max_candidates_total {
                    return Err(super::Error::Limit.into());
                }
                if diagnostic.expected_decoded_bytes.is_some() {
                    diagnostic.route = RouteState::Routed;
                }
                if range.is_empty() {
                    diagnostic.candidates.push(Candidate {
                        encoding_ordinal: encoding_ordinal as u32,
                        index_match_ordinal: None,
                        encoded_record_bytes: None,
                        observed_archive_bytes: None,
                        span: SpanState::IndexAbsent,
                        payload: Inspection::NotInspected,
                    });
                } else {
                    for (ordinal, &index) in self.index_order[range].iter().enumerate() {
                        let entry = &self.indexes[index as usize];
                        let (span, observed_archive_bytes) =
                            io::observe_span(&self.data_dir, entry, self.limits)?;
                        diagnostic.candidates.push(Candidate {
                            encoding_ordinal: encoding_ordinal as u32,
                            index_match_ordinal: Some(ordinal as u32),
                            encoded_record_bytes: Some(entry.encoded_size),
                            observed_archive_bytes,
                            span,
                            payload: Inspection::NotInspected,
                        });
                    }
                }
            }
        }
        Ok(diagnostics)
    }
}

pub(super) fn guarded<T>(
    store: &LocalStore,
    operation: impl FnOnce() -> Result<T, Error>,
) -> Result<T, Error> {
    io::check_guards(&store.guards)?;
    let result = operation();
    io::check_guards(&store.guards)?;
    result
}
