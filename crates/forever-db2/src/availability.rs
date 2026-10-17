// SPDX-License-Identifier: GPL-3.0-or-later
//! Explicit build-70374/70425 available-section projections. This module performs no
//! acquisition or cryptographic authentication. Its caller must bind a stable,
//! authenticated source whose `range` never returns synthesized hole bytes.
//! Ordinary complete-file readers and their policies are unchanged.
use crate::{RecordError, records::available_sections, terrain_sources as source};

pub const PROFILE: &str = "wowdbdefs-70374-available-sections-disabled-v1";
pub const PROFILE_70425: &str = "wowdbdefs-70425-available-sections-disabled-v1";
pub const PROFILE_70450: &str = "wowdbdefs-70450-available-sections-disabled-v1";

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
    /// Retained Vec/String capacities, excluding the source and fixed object.
    pub fn resident_bytes(&self) -> Result<usize, RecordError> {
        let rows = self
            .rows
            .capacity()
            .checked_mul(std::mem::size_of::<source::DirectRecord<source::MapFields>>())
            .ok_or(RecordError::Limit)?;
        let sections = self
            .sections
            .capacity()
            .checked_mul(std::mem::size_of::<SectionCoverage>())
            .ok_or(RecordError::Limit)?;
        self.rows.iter().try_fold(
            rows.checked_add(sections).ok_or(RecordError::Limit)?,
            |n, row| {
                n.checked_add(row.fields.name.capacity())
                    .and_then(|v| v.checked_add(row.fields.directory.capacity()))
                    .ok_or(RecordError::Limit)
            },
        )
    }
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

/// Available VMap rows with their original admitted-direct prefix and section
/// coverage. Copy replay never adds parent lookup sources or removes WDT-zero
/// rows. This owner does not validate or traverse the parent graph.
pub struct MaterializedVMapMaps {
    rows: Vec<source::DirectRecord<source::VMapFields>>,
    direct_len: usize,
    sections: Vec<SectionCoverage>,
}
impl MaterializedVMapMaps {
    /// Actual retained Vec/String capacities, excluding the borrowed source,
    /// fixed owner and temporary allocations already released by materialize.
    pub fn resident_bytes(&self) -> Result<usize, RecordError> {
        let rows = self
            .rows
            .capacity()
            .checked_mul(std::mem::size_of::<source::DirectRecord<source::VMapFields>>())
            .ok_or(RecordError::Limit)?;
        let sections = self
            .sections
            .capacity()
            .checked_mul(std::mem::size_of::<SectionCoverage>())
            .ok_or(RecordError::Limit)?;
        self.rows.iter().try_fold(
            rows.checked_add(sections).ok_or(RecordError::Limit)?,
            |n, row| {
                n.checked_add(row.fields.name.capacity())
                    .and_then(|v| v.checked_add(row.fields.directory.capacity()))
                    .ok_or(RecordError::Limit)
            },
        )
    }
    pub fn rows(&self) -> &[source::DirectRecord<source::VMapFields>] {
        &self.rows
    }
    /// Only these unchanged prefix rows may supply native-style parent lookup.
    /// Copy-created IDs in the remainder never become direct parent sources.
    pub fn direct_records(&self) -> &[source::DirectRecord<source::VMapFields>] {
        &self.rows[..self.direct_len]
    }
    pub fn sections(&self) -> &[SectionCoverage] {
        &self.sections
    }
    /// This reports declared section admission, not CKey/EKey authentication.
    pub fn all_sections_admitted(&self) -> bool {
        self.sections
            .iter()
            .all(|s| s.state == SectionState::Admitted)
    }
}
impl AvailableRecords<source::VMapFields> {
    /// Reuse the complete VMap materializer's source-direct-only copies,
    /// signed effective parents and 200k-row/64 MiB cumulative text limits.
    /// WDT-zero rows and the admitted-direct prefix remain unchanged.
    pub fn materialize(self) -> Result<MaterializedVMapMaps, RecordError> {
        let direct_len = self.records.direct_records().len();
        Ok(MaterializedVMapMaps {
            rows: self.records.materialize()?,
            direct_len,
            sections: self.sections,
        })
    }
}

/// Explicit current-catalog terrain Map projection; a missing known-section
/// byte or any required logical string dependency is fatal, not a silent skip.
pub fn read_map_70374(
    input: &impl Source,
) -> Result<AvailableRecords<source::MapFields>, RecordError> {
    available_sections::read_map(input, available_sections::Target::Build70374)
}
/// Explicit current-catalog available VMap Map projection. Omitted sections
/// stay omitted with attached coverage; required known bytes/text are fatal.
/// This cannot be converted into a public complete-file SourceRecords owner.
pub fn read_vmap_maps_70374(
    input: &impl Source,
) -> Result<AvailableRecords<source::VMapFields>, RecordError> {
    available_sections::read_vmap_maps(input, available_sections::Target::Build70374)
}
/// Original GODI consumer reads physical direct rows only. Copies are retained
/// inspection facts, never replayed or implicitly used for model acquisition.
pub fn read_gameobject_display_info_70374(
    input: &impl Source,
) -> Result<AvailableRecords<source::GameObjectFields>, RecordError> {
    available_sections::read_gameobjects(input, available_sections::Target::Build70374)
}

/// Explicit70425 terrain Map projection pinned to the70425 catalog. Sources
/// carry no build number; this binds decoder metadata, not client admission.
pub fn read_map_70425(
    input: &impl Source,
) -> Result<AvailableRecords<source::MapFields>, RecordError> {
    available_sections::read_map(input, available_sections::Target::Build70425)
}
/// Explicit70425 VMap Map projection with unchanged source-direct copy,
/// signed-parent, WDT-zero and attached section-coverage semantics.
pub fn read_vmap_maps_70425(
    input: &impl Source,
) -> Result<AvailableRecords<source::VMapFields>, RecordError> {
    available_sections::read_vmap_maps(input, available_sections::Target::Build70425)
}
/// Explicit70425 GODI projection. Copies remain inspection-only facts;
/// no model acquisition, component or product admission is performed here.
pub fn read_gameobject_display_info_70425(
    input: &impl Source,
) -> Result<AvailableRecords<source::GameObjectFields>, RecordError> {
    available_sections::read_gameobjects(input, available_sections::Target::Build70425)
}

/// Explicit70450 terrain Map projection pinned to its independent catalog.
/// Sources carry no build number; this binds decoder metadata, not admission.
pub fn read_map_70450(
    input: &impl Source,
) -> Result<AvailableRecords<source::MapFields>, RecordError> {
    available_sections::read_map(input, available_sections::Target::Build70450)
}
/// Explicit70450 VMap Map projection with unchanged source-direct copy,
/// signed-parent, WDT-zero and attached section-coverage semantics.
pub fn read_vmap_maps_70450(
    input: &impl Source,
) -> Result<AvailableRecords<source::VMapFields>, RecordError> {
    available_sections::read_vmap_maps(input, available_sections::Target::Build70450)
}
/// Explicit70450 GODI projection; no acquisition or component admission.
pub fn read_gameobject_display_info_70450(
    input: &impl Source,
) -> Result<AvailableRecords<source::GameObjectFields>, RecordError> {
    available_sections::read_gameobjects(input, available_sections::Target::Build70450)
}

#[cfg(test)]
mod vmap_capacity_tests {
    use super::*;

    #[test]
    fn available_vmap_resident_bytes_charge_retained_capacity_and_strings() {
        let mut rows = Vec::with_capacity(7);
        let mut name = String::with_capacity(32);
        name.push('n');
        let mut directory = String::with_capacity(64);
        directory.push('d');
        rows.push(source::DirectRecord {
            id: 1,
            fields: source::VMapFields {
                wdt_file_data_id: 0,
                parent_map_id: -1,
                name,
                directory,
            },
        });
        let mut sections = Vec::with_capacity(5);
        sections.push(SectionCoverage {
            ordinal: 0,
            declared_records: 1,
            declared_copies: 0,
            state: SectionState::UnknownTact,
        });
        let expected = rows.capacity()
            * std::mem::size_of::<source::DirectRecord<source::VMapFields>>()
            + rows[0].fields.name.capacity()
            + rows[0].fields.directory.capacity()
            + sections.capacity() * std::mem::size_of::<SectionCoverage>();
        let owner = MaterializedVMapMaps {
            rows,
            direct_len: 1,
            sections,
        };
        assert_eq!(owner.resident_bytes().unwrap(), expected);
        assert!(!owner.all_sections_admitted());
    }

    #[test]
    fn available_vmap_resident_bytes_charge_empty_reserved_vectors() {
        let rows = Vec::with_capacity(3);
        let sections = Vec::with_capacity(2);
        let expected = rows.capacity()
            * std::mem::size_of::<source::DirectRecord<source::VMapFields>>()
            + sections.capacity() * std::mem::size_of::<SectionCoverage>();
        let owner = MaterializedVMapMaps {
            rows,
            direct_len: 0,
            sections,
        };
        assert_eq!(owner.resident_bytes().unwrap(), expected);
        assert!(owner.direct_records().is_empty());
    }
}
