// SPDX-License-Identifier: GPL-3.0-or-later
//! Explicit build-70374 available-section projections. This module performs no
//! acquisition or cryptographic authentication. Its caller must bind a stable,
//! authenticated source whose `range` never returns synthesized hole bytes.
//! Ordinary complete-file readers and their policies are unchanged.
use crate::{RecordError, records::available_sections, terrain_sources as source};

pub const PROFILE: &str = "wowdbdefs-70374-available-sections-disabled-v1";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Unavailable;

/// Borrowed decoded-address-space capability. All requested bytes must be
/// available or the entire call fails; a short successful slice is invalid.
/// The parser checks returned lengths. No ownership, whole-buffer or zero-fill
/// conversion is part of this interface. This trait alone proves no CKey/EKey.
pub trait Source {
    fn declared_bytes(&self) -> u64;
    fn range(&self, offset: u64, bytes: usize) -> Result<&[u8], Unavailable>;
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SectionState {
    Admitted,
    /// Initial disabled-key profile admits only zero TactId. All nonzero
    /// values, including the fork's generated-output marker, are omitted.
    UnknownTact,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SectionCoverage {
    pub ordinal: u32,
    pub declared_records: u32,
    pub declared_copies: u32,
    pub state: SectionState,
}

/// Distinct partial-capable snapshot: no conversion to complete SourceRecords.
/// Direct order/duplicates and raw copy pairs from admitted sections survive.
/// Declared rows/copies in omitted sections still count against decoder caps.
pub struct AvailableRecords<T> {
    pub(crate) records: source::SourceRecords<T>,
    pub(crate) sections: Vec<SectionCoverage>,
}
impl<T> AvailableRecords<T> {
    pub fn direct_records(&self) -> &[source::DirectRecord<T>] {
        self.records.direct_records()
    }
    pub fn copies(&self) -> &[source::RecordCopy] {
        self.records.copies()
    }
    pub fn sections(&self) -> &[SectionCoverage] {
        &self.sections
    }
    pub fn all_sections_admitted(&self) -> bool {
        self.sections
            .iter()
            .all(|s| s.state == SectionState::Admitted)
    }
    pub fn decoded_utf8_bytes(&self) -> usize {
        self.records.decoded_utf8_bytes()
    }
}

/// Coverage stays attached after terrain's source-direct-only copy replay and
/// WDT-zero filtering. Finishing the admitted rows does not imply full coverage.
pub struct MaterializedMaps {
    rows: Vec<source::DirectRecord<source::MapFields>>,
    sections: Vec<SectionCoverage>,
}
impl MaterializedMaps {
    pub fn rows(&self) -> &[source::DirectRecord<source::MapFields>] {
        &self.rows
    }
    pub fn sections(&self) -> &[SectionCoverage] {
        &self.sections
    }
}
impl AvailableRecords<source::MapFields> {
    pub fn materialize(self) -> Result<MaterializedMaps, RecordError> {
        Ok(MaterializedMaps {
            rows: self.records.materialize()?,
            sections: self.sections,
        })
    }
}

/// Explicit current-catalog terrain Map projection; a missing known-section
/// byte or any required logical string dependency is fatal, not a silent skip.
pub fn read_map_70374(
    input: &impl Source,
) -> Result<AvailableRecords<source::MapFields>, RecordError> {
    available_sections::read_map(input)
}
/// Original GODI consumer reads physical direct rows only. Copies are retained
/// inspection facts, never replayed or implicitly used for model acquisition.
pub fn read_gameobject_display_info_70374(
    input: &impl Source,
) -> Result<AvailableRecords<source::GameObjectFields>, RecordError> {
    available_sections::read_gameobjects(input)
}
