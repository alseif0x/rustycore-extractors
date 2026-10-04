// SPDX-License-Identifier: GPL-3.0-or-later
//! Schema facts independently transcribed from DB2Metadata.h::{ChrClassTitleMeta,
//! ChrClassUIChrModelInfoMeta, ChrClassUIDisplayMeta, ChrClassVillainMeta,
//! ChrClassesMeta, ChrRacesMeta}, TrinityCore
//! 02245dcd245e7433e524577656177723d3e4992e. No upstream code imported.
//!
//! The four `ChrClass*` tables were verified at DB2Metadata.h:3369 (3 fields,
//! layout 0xC086F95D), :3390 (5 fields, layout 0x26E5E322), :3413 (3 fields,
//! layout 0x50F5AD66) and :3434 (3 fields, layout 0x46041630); ChrClasses at
//! :3455 (43 fields) and ChrRaces at :4182 (51 fields). Every one declares
//! `IndexField = -1` and `FieldCount = FileFieldCount`, i.e. a separate id table
//! and no appended parent field.
//!
//! The two leading ChrRaces fields are FT_STRING_NOT_LOCALIZED in the reference
//! (DB2Metadata.h:4182); this implementation decodes both as ordinary text, so
//! they map to the same `Kind::String` with no behavior difference.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CharacterTable {
    Classes,
    Races,
    ClassTitle,
    ClassUIDisplay,
    ClassUIChrModelInfo,
    ClassVillain,
}
impl CharacterTable {
    pub fn named(name: &str) -> Option<Self> {
        match name {
            "ChrClasses" => Some(Self::Classes),
            "ChrRaces" => Some(Self::Races),
            "ChrClassTitle" => Some(Self::ClassTitle),
            "ChrClassUIDisplay" => Some(Self::ClassUIDisplay),
            "ChrClassUIChrModelInfo" => Some(Self::ClassUIChrModelInfo),
            "ChrClassVillain" => Some(Self::ClassVillain),
            _ => None,
        }
    }
    pub fn name(self) -> &'static str {
        match self {
            Self::Classes => "ChrClasses",
            Self::Races => "ChrRaces",
            Self::ClassTitle => "ChrClassTitle",
            Self::ClassUIDisplay => "ChrClassUIDisplay",
            Self::ClassUIChrModelInfo => "ChrClassUIChrModelInfo",
            Self::ClassVillain => "ChrClassVillain",
        }
    }
    pub fn file_id(self) -> u32 {
        match self {
            Self::Classes => 1361031,
            Self::Races => 1305311,
            Self::ClassTitle => 1267497,
            Self::ClassUIDisplay => 1316515,
            Self::ClassUIChrModelInfo => 3754770,
            Self::ClassVillain => 1267498,
        }
    }
    /// True only when the header's identity fields match the reviewed schema:
    /// layout hash, field count and total field count, plus the table hash when
    /// the reviewed metadata can supply one.
    ///
    /// The native header's `IndexField` is deliberately NOT part of this
    /// comparison. Reference `DB2FileLoader.cpp:1746` only endian-converts
    /// `_header.IndexField` and never consults it; whether the index lives in
    /// the record body vs. a separate id table is decided from the reviewed
    /// metadata via `Meta->HasIndexFieldInData()` (`DB2Meta.cpp:22-25`,
    /// `Meta->IndexField != -1`), i.e. this schema's own `index`, not the
    /// header word. Real 70205 evidence confirms the two disagree (ChrRaces
    /// header `IndexField == 0` while the reviewed meta index is `-1`).
    ///
    /// `table_hash` is checked only when `schema().hash` is `Some`. The pinned
    /// reference `DB2Meta` (`src/common/DataStores/DB2Meta.h:32-49`) declares
    /// `FileDataId, IndexField, ParentIndexField, FieldCount, FileFieldCount,
    /// LayoutHash, Fields` and has NO table-hash member, so `DB2Metadata.h`
    /// cannot supply it for any table. Likewise the loader validates
    /// `_header.LayoutHash == Meta->LayoutHash` (`DB2FileLoader.cpp:1763`) and
    /// `TotalFieldCount` against `Meta->FieldCount` (`:1771`) but never compares
    /// `_header.TableHash`; `GetTableHash()` merely returns the header word
    /// (`DB2FileLoader.h:209`). Transcribed table hashes are therefore retained
    /// only where real-header evidence pinned them (ChrClasses/ChrRaces); for
    /// the four `ChrClass*` tables the gate stays on the fields the reference
    /// actually checks. No table hash is invented.
    pub fn matches(self, header: &crate::Header) -> bool {
        let schema = self.schema();
        let table_hash_matches = schema.hash.is_none_or(|hash| header.table_hash == hash);
        table_hash_matches
            && header.layout_hash == schema.layout
            && header.field_count as usize == schema.fields.len()
            && header.total_field_count == header.field_count
    }
    /// Reviewed schema identity `(table_hash, layout_hash, index, field_count)`.
    /// The `index` element is the authoritative metadata index used for inline
    /// vs. id-table decoding; it is NOT a gate and does not describe the header
    /// word. When the reference metadata cannot supply a table hash the first
    /// element is `0`, the same "unknown" sentinel used by the batch fact path.
    pub fn schema_identity(self) -> (u32, u32, i16, usize) {
        let schema = self.schema();
        (
            schema.hash.unwrap_or(0),
            schema.layout,
            schema.index,
            schema.fields.len(),
        )
    }
    pub(super) fn schema(self) -> Schema {
        use Kind::*;
        match self {
            Self::ClassTitle => Schema {
                hash: None,
                layout: 0xC086F95D,
                index: -1,
                fields: &[String, String, Signed(8, 1)],
            },
            Self::ClassUIDisplay => Schema {
                hash: None,
                layout: 0x50F5AD66,
                index: -1,
                fields: &[Signed(8, 1), Unsigned(32, 1), Unsigned(32, 1)],
            },
            Self::ClassUIChrModelInfo => Schema {
                hash: None,
                layout: 0x26E5E322,
                index: -1,
                fields: &[
                    Float(3),
                    Signed(32, 1),
                    Signed(8, 1),
                    Float(1),
                    Signed(32, 1),
                ],
            },
            Self::ClassVillain => Schema {
                hash: None,
                layout: 0x46041630,
                index: -1,
                fields: &[String, Signed(8, 1), Signed(8, 1)],
            },
            Self::Classes => Schema {
                hash: Some(0xF5889D8C),
                layout: 0xAFC9B0C2,
                index: 29,
                fields: &[
                    String,
                    String,
                    String,
                    String,
                    String,
                    String,
                    String,
                    String,
                    String,
                    String,
                    Unsigned(32, 1),
                    Unsigned(32, 1),
                    Unsigned(32, 1),
                    Unsigned(32, 1),
                    Signed(32, 1),
                    Signed(32, 1),
                    Unsigned(32, 1),
                    Unsigned(32, 1),
                    Signed(32, 1),
                    Signed(32, 1),
                    Signed(32, 1),
                    Signed(32, 1),
                    Signed(32, 1),
                    Signed(32, 1),
                    Signed(32, 1),
                    Signed(32, 1),
                    Signed(32, 1),
                    Unsigned(16, 1),
                    Unsigned(16, 1),
                    Unsigned(8, 1),
                    Unsigned(8, 1),
                    Signed(8, 1),
                    Signed(8, 1),
                    Unsigned(8, 1),
                    Unsigned(8, 1),
                    Unsigned(8, 1),
                    Unsigned(8, 1),
                    Unsigned(8, 1),
                    Unsigned(8, 1),
                    Unsigned(8, 1),
                    Unsigned(8, 1),
                    Unsigned(8, 1),
                    Unsigned(8, 1),
                ],
            },
            Self::Races => Schema {
                hash: Some(0x53F1783C),
                layout: 0x4F44C796,
                index: -1,
                fields: &[
                    String,
                    String,
                    String,
                    String,
                    String,
                    String,
                    String,
                    String,
                    String,
                    String,
                    String,
                    String,
                    String,
                    String,
                    String,
                    Signed(32, 1),
                    Signed(32, 1),
                    Signed(32, 1),
                    Signed(32, 1),
                    Signed(32, 1),
                    Signed(32, 1),
                    Signed(32, 1),
                    Signed(32, 1),
                    Signed(32, 3),
                    Signed(32, 3),
                    Signed(32, 1),
                    Signed(32, 1),
                    Signed(32, 1),
                    Signed(32, 1),
                    Signed(32, 1),
                    Float(3),
                    Float(1),
                    Float(3),
                    Float(3),
                    Signed(8, 1),
                    Unsigned(8, 1),
                    Signed(8, 1),
                    Signed(8, 1),
                    Signed(8, 1),
                    Signed(8, 1),
                    Signed(8, 1),
                    Signed(8, 1),
                    Signed(8, 1),
                    Signed(8, 1),
                    Signed(8, 1),
                    Signed(8, 1),
                    Signed(8, 1),
                    Signed(8, 1),
                    Signed(8, 1),
                    Signed(8, 1),
                    Signed(8, 1),
                ],
            },
        }
    }
}
pub(super) struct Schema {
    /// Transcribed table hash when real-header evidence pinned it; `None` when
    /// the pinned `DB2Meta` cannot supply one (see `matches`). Never invented.
    pub hash: Option<u32>,
    pub layout: u32,
    pub index: i16,
    pub fields: &'static [Kind],
}
#[derive(Clone, Copy)]
pub(super) enum Kind {
    String,
    Unsigned(u32, usize),
    Signed(u32, usize),
    Float(usize),
}
impl Kind {
    pub fn bits(self) -> u32 {
        match self {
            Self::Unsigned(bits, _) | Self::Signed(bits, _) => bits,
            _ => 32,
        }
    }
    pub fn count(self) -> usize {
        match self {
            Self::Unsigned(_, count) | Self::Signed(_, count) | Self::Float(count) => count,
            _ => 1,
        }
    }
}
