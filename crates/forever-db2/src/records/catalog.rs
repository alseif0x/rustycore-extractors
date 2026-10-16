// SPDX-License-Identifier: GPL-3.0-or-later
// Added 2026-10-11: independent metadata catalog; no upstream code imported.
//! Complete separately pinned factual catalogs for different client targets.
//! Build-specific catalogs retain separate provenance even where their reviewed
//! table facts match another target. None imports upstream code.
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
#[path = "wowdbdefs_70205_generated.rs"]
mod wowdbdefs_70205_generated;
#[path = "wowdbdefs_70338_generated.rs"]
mod wowdbdefs_70338_generated;
#[path = "wowdbdefs_70374_generated.rs"]
mod wowdbdefs_70374_generated;
#[path = "wowdbdefs_70425_generated.rs"]
mod wowdbdefs_70425_generated;

pub const CATALOG_ID: &str = "pinned-metadata-v1";
pub const SOURCE_COMMIT: &str = "02245dcd245e7433e524577656177723d3e4992e";
pub const SOURCE_PATH: &str = "src/server/game/DataStores/DB2Metadata.h";
pub const SOURCE_GIT_BLOB: &str = "d06b97bad72ca2aecad3987ce9c348ac6055e0db";
pub const SOURCE_SHA256: &str = "897bde5aa04bab3b40ce4fe2e34d2127385b11d908d5266dce0fae7b52e23def";
pub const CATALOG_FACTS_SHA256: &str =
    "0f1ed0c419e851078d99f46ad3b37cadddb27b0556a2d1b52d3fd9ab62b76ec9";
pub const WOWDBDEFS_70205_CATALOG_ID: &str = "wowdbdefs-70205-v1";
pub const WOWDBDEFS_70205_SOURCE_COMMIT: &str = "674636522d51c010f250ad1b5fe549ef6bc8eb0a";
pub const WOWDBDEFS_70205_SOURCE_BLOB: &str = "8a5d8122f312ae6a9564e8286fdc491831fecc22";
pub const WOWDBDEFS_70205_SOURCE_SHA256: &str =
    "4314266fe055e30207e1c39498a7a31058a1aa2ab6cd674726491b313d135265";
pub const WOWDBDEFS_70205_FACTS_SHA256: &str =
    "fc8a2d07e18931be37a269e018e57ab00df6bd3b19794f2a025d06eaa280521b";

pub const WOWDBDEFS_70338_CATALOG_ID: &str = "wowdbdefs-70338-v1";
pub const WOWDBDEFS_70338_SOURCE_COMMIT: &str = "d6f50d4f351193e84226aa62a36faa0f58528afa";
pub const WOWDBDEFS_70338_SOURCE_BLOB: &str = "8a5d8122f312ae6a9564e8286fdc491831fecc22";
pub const WOWDBDEFS_70338_SOURCE_SHA256: &str =
    "4314266fe055e30207e1c39498a7a31058a1aa2ab6cd674726491b313d135265";
pub const WOWDBDEFS_70338_FACTS_SHA256: &str =
    "9fb8852f7ec15a5721ff17da4708970eacb54c5e79c4e78703891a2468905405";

// Added 2026-10-14: independently generated build-70374 public schema facts.
pub const WOWDBDEFS_70374_CATALOG_ID: &str = "wowdbdefs-70374-v1";
pub const WOWDBDEFS_70374_SOURCE_COMMIT: &str = "464a25b3a7c02b0c74726be7a53a0b0ccf1a2d10";
pub const WOWDBDEFS_70374_SOURCE_BLOB: &str = "8a5d8122f312ae6a9564e8286fdc491831fecc22";
pub const WOWDBDEFS_70374_SOURCE_SHA256: &str =
    "4314266fe055e30207e1c39498a7a31058a1aa2ab6cd674726491b313d135265";
pub const WOWDBDEFS_70374_FACTS_SHA256: &str =
    "cf9a5a549136090c3aba1dddccc76c4aec45eb8a30162f053860f2bed44d2faf";

// Separate build-70425 public schema facts; no acquisition or runtime acceptance.
pub const WOWDBDEFS_70425_CATALOG_ID: &str = "wowdbdefs-70425-v1";
pub const WOWDBDEFS_70425_SOURCE_COMMIT: &str = "996e53f029c0213b223aecb94d754f6b4b9efb49";
pub const WOWDBDEFS_70425_SOURCE_BLOB: &str = "f95528066d44e1b70ea6cd124a35a5a63685d384";
pub const WOWDBDEFS_70425_SOURCE_SHA256: &str =
    "f7b752a991d6a0321d07ecb7f06e62cd94694c1594d2fe5ce0cb7b243c71559a";
pub const WOWDBDEFS_70425_FACTS_SHA256: &str =
    "2150cbfc6a431931d410c3368e5e7b8de2c66b6b6a798d0052886f948624488a";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CatalogProvenance {
    pub id: &'static str,
    pub source_commit: &'static str,
    pub source_blob: &'static str,
    pub source_sha256: &'static str,
    pub facts_sha256: &'static str,
}

pub fn provenance(catalog_id: &str) -> Option<CatalogProvenance> {
    match catalog_id {
        CATALOG_ID => Some(CatalogProvenance {
            id: CATALOG_ID,
            source_commit: SOURCE_COMMIT,
            source_blob: SOURCE_GIT_BLOB,
            source_sha256: SOURCE_SHA256,
            facts_sha256: CATALOG_FACTS_SHA256,
        }),
        WOWDBDEFS_70205_CATALOG_ID => Some(CatalogProvenance {
            id: WOWDBDEFS_70205_CATALOG_ID,
            source_commit: WOWDBDEFS_70205_SOURCE_COMMIT,
            source_blob: WOWDBDEFS_70205_SOURCE_BLOB,
            source_sha256: WOWDBDEFS_70205_SOURCE_SHA256,
            facts_sha256: WOWDBDEFS_70205_FACTS_SHA256,
        }),
        WOWDBDEFS_70338_CATALOG_ID => Some(CatalogProvenance {
            id: WOWDBDEFS_70338_CATALOG_ID,
            source_commit: WOWDBDEFS_70338_SOURCE_COMMIT,
            source_blob: WOWDBDEFS_70338_SOURCE_BLOB,
            source_sha256: WOWDBDEFS_70338_SOURCE_SHA256,
            facts_sha256: WOWDBDEFS_70338_FACTS_SHA256,
        }),
        WOWDBDEFS_70374_CATALOG_ID => Some(CatalogProvenance {
            id: WOWDBDEFS_70374_CATALOG_ID,
            source_commit: WOWDBDEFS_70374_SOURCE_COMMIT,
            source_blob: WOWDBDEFS_70374_SOURCE_BLOB,
            source_sha256: WOWDBDEFS_70374_SOURCE_SHA256,
            facts_sha256: WOWDBDEFS_70374_FACTS_SHA256,
        }),
        WOWDBDEFS_70425_CATALOG_ID => Some(CatalogProvenance {
            id: WOWDBDEFS_70425_CATALOG_ID,
            source_commit: WOWDBDEFS_70425_SOURCE_COMMIT,
            source_blob: WOWDBDEFS_70425_SOURCE_BLOB,
            source_sha256: WOWDBDEFS_70425_SOURCE_SHA256,
            facts_sha256: WOWDBDEFS_70425_FACTS_SHA256,
        }),
        _ => None,
    }
}

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

/// Catalog compatibility reasons. Neither current catalog has blocked value
/// shapes; unsupported file variants remain separate decoder errors.
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
    /// Exact table hash when the source declares one; absent in the legacy
    /// TrinityCore metadata catalog.
    pub table_hash: Option<u32>,
    /// Additional source-accepted layout hashes for this same schema.
    pub other_layout_hashes: &'static [u32],
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
enum CatalogSource {
    PinnedMetadata,
    WowDbDefs70205,
    WowDbDefs70338,
    WowDbDefs70374,
    WowDbDefs70425,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CatalogTable {
    source: CatalogSource,
    index: u16,
}

impl CatalogTable {
    pub fn metadata(self) -> &'static TableMetadata {
        &self.tables()[usize::from(self.index)]
    }

    fn tables(self) -> &'static [TableMetadata] {
        match self.source {
            CatalogSource::PinnedMetadata => generated::TABLES,
            CatalogSource::WowDbDefs70205 => wowdbdefs_70205_generated::TABLES,
            CatalogSource::WowDbDefs70338 => wowdbdefs_70338_generated::TABLES,
            CatalogSource::WowDbDefs70374 => wowdbdefs_70374_generated::TABLES,
            CatalogSource::WowDbDefs70425 => wowdbdefs_70425_generated::TABLES,
        }
    }

    pub fn catalog_id(self) -> &'static str {
        match self.source {
            CatalogSource::PinnedMetadata => CATALOG_ID,
            CatalogSource::WowDbDefs70205 => WOWDBDEFS_70205_CATALOG_ID,
            CatalogSource::WowDbDefs70338 => WOWDBDEFS_70338_CATALOG_ID,
            CatalogSource::WowDbDefs70374 => WOWDBDEFS_70374_CATALOG_ID,
            CatalogSource::WowDbDefs70425 => WOWDBDEFS_70425_CATALOG_ID,
        }
    }

    pub(super) fn matches_layout(self, layout_hash: u32) -> bool {
        let metadata = self.metadata();
        metadata.layout_hash == layout_hash || metadata.other_layout_hashes.contains(&layout_hash)
    }

    pub(super) fn schema(self) -> Schema {
        let metadata = self.metadata();
        // The legacy catalog keeps hand-reviewed overrides for callers that
        // wrap a public CatalogTable directly. The build-specific catalog must
        // always decode from its own pinned DBD facts, even when a name also
        // appears in the older reviewed enum.
        if self.source == CatalogSource::PinnedMetadata
            && let Some(reviewed) = super::CharacterTable::named(metadata.name)
        {
            return reviewed.schema();
        }
        Schema {
            hash: metadata.table_hash,
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

pub fn tables_for(catalog_id: &str) -> Option<&'static [TableMetadata]> {
    match catalog_id {
        CATALOG_ID => Some(generated::TABLES),
        WOWDBDEFS_70205_CATALOG_ID => Some(wowdbdefs_70205_generated::TABLES),
        WOWDBDEFS_70338_CATALOG_ID => Some(wowdbdefs_70338_generated::TABLES),
        WOWDBDEFS_70374_CATALOG_ID => Some(wowdbdefs_70374_generated::TABLES),
        WOWDBDEFS_70425_CATALOG_ID => Some(wowdbdefs_70425_generated::TABLES),
        _ => None,
    }
}

pub fn supports(catalog_id: &str) -> bool {
    tables_for(catalog_id).is_some()
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
    resolve_for(CATALOG_ID, name)
}

pub fn resolve_for(catalog_id: &str, name: &str) -> Result<Option<CatalogTable>, CatalogError> {
    let (source, tables) = match catalog_id {
        CATALOG_ID => (CatalogSource::PinnedMetadata, generated::TABLES),
        WOWDBDEFS_70205_CATALOG_ID => (
            CatalogSource::WowDbDefs70205,
            wowdbdefs_70205_generated::TABLES,
        ),
        WOWDBDEFS_70338_CATALOG_ID => (
            CatalogSource::WowDbDefs70338,
            wowdbdefs_70338_generated::TABLES,
        ),
        WOWDBDEFS_70374_CATALOG_ID => (
            CatalogSource::WowDbDefs70374,
            wowdbdefs_70374_generated::TABLES,
        ),
        WOWDBDEFS_70425_CATALOG_ID => (
            CatalogSource::WowDbDefs70425,
            wowdbdefs_70425_generated::TABLES,
        ),
        _ => return Ok(None),
    };
    let Ok(index) = tables.binary_search_by_key(&name, |metadata| metadata.name) else {
        return Ok(None);
    };
    let metadata = &tables[index];
    if !metadata.shape_compatible() {
        return Err(CatalogError {
            name: metadata.name,
            blockers: metadata.blockers,
        });
    }
    Ok(Some(CatalogTable {
        source,
        index: index as u16,
    }))
}
