// SPDX-License-Identifier: GPL-3.0-or-later
// Added 2026-10-11: independent metadata catalog; no upstream code imported.
//! Complete factual catalog from one verified upstream metadata declaration.
//!
//! Shape compatibility describes the current bounded decoder's value model.
//! It does not promise readable CASC data, supported WDC5 section variants,
//! extraction parity or RustyCore reader/runtime acceptance. The existing
//! reviewed lookup and two-table default remain separate, explicit policies.
use super::schema::{Kind, Schema};

#[path = "catalog_generated.rs"]
mod generated;
#[cfg(test)]
#[path = "catalog_tests.rs"]
mod tests;

pub const CATALOG_ID: &str = "pinned-metadata-v1";
pub const SOURCE_COMMIT: &str = "02245dcd245e7433e524577656177723d3e4992e";
pub const SOURCE_PATH: &str = "src/server/game/DataStores/DB2Metadata.h";
pub const SOURCE_GIT_BLOB: &str = "d06b97bad72ca2aecad3987ce9c348ac6055e0db";
pub const SOURCE_SHA256: &str = "897bde5aa04bab3b40ce4fe2e34d2127385b11d908d5266dce0fae7b52e23def";

/// Source declaration type, before decoder normalization. In particular,
/// localized and nonlocalized text remain distinct metadata facts.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FieldType {
    String,
    NonLocalizedString,
    Byte,
    Short,
    Int,
    Long,
    Float,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FieldMetadata {
    pub kind: FieldType,
    pub array_size: u8,
    pub signed: bool,
}

/// Catalog compatibility reasons. The current pinned catalog has no blocked
/// value shapes; unsupported file variants remain separate decoder errors.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CatalogBlocker {
    Signed64Bit,
    StringArray,
    UnsignedAppendedParent,
}

impl CatalogBlocker {
    pub fn code(self) -> &'static str {
        match self {
            Self::Signed64Bit => "signed_64_bit",
            Self::StringArray => "string_array",
            Self::UnsignedAppendedParent => "unsigned_appended_parent",
        }
    }
}

/// Complete source metadata, including the appended parent when declared.
/// `file_field_count` identifies the physical prefix of `fields`.
#[derive(Debug)]
pub struct TableMetadata {
    pub name: &'static str,
    pub file_data_id: u32,
    pub layout_hash: u32,
    pub index: i16,
    pub parent_index: i16,
    pub field_count: usize,
    pub file_field_count: usize,
    pub fields: &'static [FieldMetadata],
    pub blockers: &'static [CatalogBlocker],
    decode_fields: Option<&'static [Kind]>,
}

impl TableMetadata {
    pub fn shape_compatible(&self) -> bool {
        self.blockers.is_empty()
    }

    /// Native record storage normalizes every appended byte/short/int parent
    /// to 32 bits (DB2Meta.cpp::GetRecordSize, DB2FileLoader::FillParentLookup).
    /// The raw declaration above is retained; source signedness chooses the
    /// decoder's signed or unsigned 32-bit appended value.
    pub fn appended_parent_storage_bits(&self) -> Option<u32> {
        (self.parent_index >= self.file_field_count as i16).then_some(32)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CatalogError {
    pub name: &'static str,
    pub blockers: &'static [CatalogBlocker],
}

impl CatalogError {
    pub fn code(self) -> &'static str {
        "unsupported_catalog_schema"
    }
}

impl std::fmt::Display for CatalogError {
    fn fmt(&self, out: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(out, "{}: {}", self.code(), self.name)
    }
}
impl std::error::Error for CatalogError {}

/// An unforgeable handle to a shape-compatible descriptor. Construct it via
/// `resolve`, or use `CharacterTable::from_catalog` to retain reviewed gates.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CatalogTable(u16);

impl CatalogTable {
    pub fn metadata(self) -> &'static TableMetadata {
        &generated::TABLES[usize::from(self.0)]
    }

    pub(super) fn schema(self) -> Schema {
        let metadata = self.metadata();
        // A caller can wrap a public handle in CharacterTable::Catalog;
        // that route must retain reviewed table-hash gates as well.
        if let Some(reviewed) = super::CharacterTable::named(metadata.name) {
            return reviewed.schema();
        }
        Schema {
            hash: None,
            layout: metadata.layout_hash,
            index: metadata.index,
            parent: metadata.parent_index,
            appended_parent_unsigned: metadata.appended_parent_storage_bits().is_some()
                && !metadata.fields[metadata.parent_index as usize].signed,
            fields: metadata.decode_fields.expect("validated catalog handle"),
        }
    }
}

pub fn tables() -> &'static [TableMetadata] {
    generated::TABLES
}

pub fn find(name: &str) -> Option<&'static TableMetadata> {
    generated::TABLES
        .binary_search_by_key(&name, |metadata| metadata.name)
        .ok()
        .map(|index| &generated::TABLES[index])
}

/// Explicit catalog lookup. Known unsupported value shapes are errors, not
/// unknown names. No input file is read and no availability is inferred.
pub fn resolve(name: &str) -> Result<Option<CatalogTable>, CatalogError> {
    let Ok(index) = generated::TABLES.binary_search_by_key(&name, |metadata| metadata.name) else {
        return Ok(None);
    };
    let metadata = &generated::TABLES[index];
    if !metadata.shape_compatible() {
        return Err(CatalogError {
            name: metadata.name,
            blockers: metadata.blockers,
        });
    }
    Ok(Some(CatalogTable(index as u16)))
}
