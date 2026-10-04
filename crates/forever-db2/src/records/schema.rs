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
//!
//! `ChrClassesXPowerTypes` is the one reviewed table with a parent: it declares
//! `IndexField = -1`, `ParentIndexField = 1`, `FieldCount = 2`,
//! `FileFieldCount = 1` and `LayoutHash 0x70DA1F8C` (DB2Metadata.h). Its parent
//! value is an extra row field appended after the single column field and is
//! filled from the separate parent-lookup block.
//!
//! `Schema` separates the two counts the reference distinguishes:
//! - `fields` is the decoded column list, one column per `FileFieldCount`; the
//!   header's `field_count` equals `fields.len()`.
//! - `parent` is the reviewed `ParentIndexField`, an index into the TOTAL field
//!   array (columns plus any appended parent). When `parent >= fields.len()` the
//!   parent is the "extra field at the end" variant: it has no column meta entry
//!   and `DB2FileLoader.cpp:1771` then requires
//!   `total_field_count + 1 == Meta->FieldCount`. For `ChrClassesXPowerTypes`
//!   that gives `total_field_count = 1` while `field_count = 1` and
//!   `ParentIndexField = 1` (the append slice index).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CharacterTable {
    Classes,
    Races,
    ClassTitle,
    ClassUIDisplay,
    ClassUIChrModelInfo,
    ClassVillain,
    /// `ChrClassesXPowerTypes`: the one reviewed table that declares a
    /// `ParentIndexField == 1`. With `FileFieldCount == 1` the parent index sits
    /// past the column list (`1 >= fields.len()`), so the parent value is the
    /// "extra field at the end": NOT a column entry in the record body, filled
    /// from the separate parent-lookup block after the copy table.
    ClassXPowerTypes,
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
            "ChrClassesXPowerTypes" => Some(Self::ClassXPowerTypes),
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
            Self::ClassXPowerTypes => "ChrClassesXPowerTypes",
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
            Self::ClassXPowerTypes => 1121420,
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
    /// A parent-bearing table declares one extra row field beyond its column
    /// meta when `ParentIndexField >= FileFieldCount` (the "extra field at the
    /// end" variant). Reference `DB2FileLoader.cpp:1771` then requires
    /// `_header.TotalFieldCount + (ParentIndexField >= TotalFieldCount ? 1 : 0)
    /// == Meta->FieldCount`, i.e. `total_field_count + 1 == FieldCount` for this
    /// variant. So `ClassXPowerTypes` has header `field_count = 1` (the single
    /// column) and `total_field_count = 1`, with the reviewed `FieldCount = 2`
    /// column list describing one decoded column plus the appended parent.
    /// Tables without a parent keep `total_field_count == field_count`.
    pub fn matches(self, header: &crate::Header) -> bool {
        let schema = self.schema();
        let table_hash_matches = schema.hash.is_none_or(|hash| header.table_hash == hash);
        // The header's `field_count` is the column count (FileFieldCount).
        let field_count_matches = header.field_count as usize == schema.fields.len();
        // DB2FileLoader.cpp:1771: the gate is
        //   TotalFieldCount + (ParentIndexField >= TotalFieldCount ? 1 : 0)
        //   == Meta->FieldCount.
        // For the reviewed extra-field-at-the-end variant this is
        //   total_field_count + 1 == fields.len() + 1,
        // so `total_field_count` must equal the column count. No-parent tables
        // keep `total_field_count == field_count`.
        let total_field_count_matches =
            if schema.parent >= 0 && (schema.parent as usize) >= schema.fields.len() {
                header.total_field_count as usize + 1 == schema.fields.len() + 1
            } else {
                header.total_field_count == header.field_count
            };
        table_hash_matches
            && header.layout_hash == schema.layout
            && field_count_matches
            && total_field_count_matches
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
                parent: -1,
                fields: &[String, String, Signed(8, 1)],
            },
            Self::ClassUIDisplay => Schema {
                hash: None,
                layout: 0x50F5AD66,
                index: -1,
                parent: -1,
                fields: &[Signed(8, 1), Unsigned(32, 1), Unsigned(32, 1)],
            },
            Self::ClassUIChrModelInfo => Schema {
                hash: None,
                layout: 0x26E5E322,
                index: -1,
                parent: -1,
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
                parent: -1,
                fields: &[String, Signed(8, 1), Signed(8, 1)],
            },
            // Transcribed from DB2Metadata.h::ChrClassesXPowerTypesMeta:
            // FileDataId 1121420, IndexField -1, ParentIndexField 1, FieldCount 2,
            // FileFieldCount 1, LayoutHash 0x70DA1F8C, one signed byte column.
            // The parent value at total-array position 1 is the extra field
            // `DB2Metadata.h` marks with `PARENT_INDEX_FIELD`; it has NO column
            // meta entry and is filled from the parent-lookup block.
            Self::ClassXPowerTypes => Schema {
                hash: None,
                layout: 0x70DA1F8C,
                index: -1,
                parent: 1,
                fields: &[Signed(8, 1)],
            },
            Self::Classes => Schema {
                hash: Some(0xF5889D8C),
                layout: 0xAFC9B0C2,
                index: 29,
                parent: -1,
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
                parent: -1,
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
    /// Reviewed metadata inline index; `-1` means a separate id table.
    pub index: i16,
    /// Reviewed `ParentIndexField`; `-1` means the table has no parent field.
    /// The index is into the TOTAL field array (columns plus any appended
    /// parent). When it is `>= fields.len()` (the "extra field at the end"
    /// variant) the header's `total_field_count` satisfies `total + 1 ==
    /// FieldCount` per `DB2FileLoader.cpp:1771`, and the value is filled from
    /// the separate parent-lookup block, not from a column meta entry.
    pub parent: i16,
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
