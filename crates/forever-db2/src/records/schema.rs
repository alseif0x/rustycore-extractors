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
//! `ChrSpecialization` is the first reviewed table whose parent index lies
//! INSIDE the file field count: `IndexField 3`, `ParentIndexField 4`,
//! `FieldCount = FileFieldCount = 13`, FileDataId 1343390 and
//! `LayoutHash 0xDAB4CA4B`. The parent lookup overwrites the existing signed
//! 8-bit column 4 in the data block (with the field's own width) instead of an
//! appended field; a ParentId that does not fit that width is rejected.
//!
//! Five further Forever tables were reviewed at DB2Metadata.h:
//! `ChrCustomizationCategory` (FileDataId 3526439, 8 fields, layout 0x1605B68A),
//! `ChrCustItemGeoModify` (3952714, 3 fields, layout 0x2986E1F3),
//! `ChrCustomizationGeoset` (3456171, 3 fields, layout 0xBF55F2FB),
//! `ChrCustomizationMaterial` (3459652, 2 fields, layout 0xBE9767E9) and
//! `ChrCustomizationBoneSet` (3460181, 2 fields, layout 0xCC2CFF6E). All five
//! declare `IndexField = -1`, `ParentIndexField = -1` and
//! `FieldCount == FileFieldCount` (no appended parent field).
//!
//! Three further Forever tables were reviewed at DB2Metadata.h:
//! `ChrCustomizationElement` (FileDataId 3512765, 13 fields, layout 0x6483C37E),
//! `ChrCustomizationDisplayInfo` (3593032, 5 fields, layout 0xB24CE54F) and
//! `ChrCustomizationReq` (3450453, 9 fields, layout 0xCA154412). All three
//! declare `IndexField = -1`, `ParentIndexField = -1` and
//! `FieldCount == FileFieldCount`. `ChrCustomizationDisplayInfo` has two signed
//! 32-bit fields then three floats; `ChrCustomizationReq` is a string, seven
//! signed 32-bit fields and one signed 32-bit array of two.
//!
//! Three further Forever tables were reviewed at DB2Metadata.h:
//! `Curve` (FileDataId 892585, 3 fields, layout 0xD362535A),
//! `ContentRestrictionRuleSet` (4281307, 3 fields, layout 0x5774D75A) and
//! `DataTagInfo` (6652252, 3 fields, layout 0xDB46F772). All three declare
//! `IndexField >= 0` (0, 0 and 1 respectively), `ParentIndexField = -1` and
//! `FieldCount == FileFieldCount`, i.e. the record id is read from an inline
//! column rather than a separate id table. `Curve` is [Unsigned(32,1),
//! Unsigned(8,1), Signed(32,1)] (the second field is an UNSIGNED FT_BYTE),
//! `ContentRestrictionRuleSet` is one unsigned 32-bit field (the inline index)
//! followed by two signed 32-bit fields and `DataTagInfo`
//! is a string followed by two signed 32-bit fields.
//!
//! Three further Forever tables were reviewed at DB2Metadata.h:
//! `AnimKitBoneSetAlias` (FileDataId 1266937, 2 fields, layout 0x19A6986C),
//! `AnimKitPriority` (1266540, 1 field, layout 0xCCF889D8) and
//! `AnimReplacementSet` (1284817, 1 field, layout 0x7C047695). All three declare
//! `IndexField = -1`, `ParentIndexField = -1` and `FieldCount == FileFieldCount`,
//! i.e. a separate id table and no appended parent field. `AnimKitBoneSetAlias`
//! is two unsigned FT_BYTE fields; the other two are each a single unsigned
//! FT_BYTE field. These are the first reviewed tables with a one-field file
//! field count.
//!
//! Three further Forever tables were reviewed at DB2Metadata.h:
//! `AnimKitBoneSet` (FileDataId 1375433, 5 fields, layout 0x43E7736F),
//! `AnimationData` (1375431, 4 fields, layout 0xBBF66A3C) and
//! `AnimKitConfig` (1300872, 1 field, layout 0x140718EF). All three declare
//! `IndexField = -1`, `ParentIndexField = -1` and `FieldCount ==
//! FileFieldCount`, i.e. a separate id table and no appended parent field.
//! `AnimKitBoneSet` is [String, Signed(32,1), Signed(8,1), Signed(8,1),
//! Signed(32,1)] (its leading field is `FT_STRING_NOT_LOCALIZED`, which decodes
//! as ordinary text); `AnimationData` is [Unsigned(16,1), Signed(8,1),
//! Signed(16,1), Signed(32,2)], adding a signed 16-bit field and a signed 32-bit
//! array of two; `AnimKitConfig` is a single signed 32-bit field.
//!
//! Three further Forever tables were reviewed at DB2Metadata.h and are the first
//! to use an UNSIGNED 64-bit `FT_LONG` field (read as 8 little-endian bytes in
//! compression 0): `ItemExtendedCost` (FileDataId 801681, 11 fields, layout
//! 0x22331DAB, IndexField -1), `TransmogOutfitEntry` (7320164, 9 fields, layout
//! 0x91D51304, IndexField 2) and `BankTab` (5242411, 9 fields, layout
//! 0x5B2F9B36, IndexField -1). All declare `ParentIndexField = -1` and
//! `FieldCount == FileFieldCount`.
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
    /// `ChrCustomizationCategory`: one string plus six signed 32-bit fields and
    /// one float. No parent (`ParentIndexField == -1`), separate u32 id table.
    CustomizationCategory,
    /// `ChrCustItemGeoModify`: one signed byte plus two signed 32-bit fields.
    CustItemGeoModify,
    /// `ChrCustomizationGeoset`: one signed byte plus two signed 32-bit fields.
    CustomizationGeoset,
    /// `ChrCustomizationMaterial`: two signed 32-bit fields.
    CustomizationMaterial,
    /// `ChrCustomizationBoneSet`: two signed 32-bit fields.
    CustomizationBoneSet,
    /// `ChrCustomizationElement`: thirteen signed 32-bit fields.
    CustomizationElement,
    /// `ChrCustomizationDisplayInfo`: two signed 32-bit fields followed by
    /// three floats.
    CustomizationDisplayInfo,
    /// `ChrCustomizationReq`: one string, seven signed 32-bit fields and one
    /// signed 32-bit array of two.
    CustomizationReq,
    /// `Curve`: one unsigned 32-bit field (the inline index), one unsigned
    /// 8-bit field and one signed 32-bit field.
    Curve,
    /// `ContentRestrictionRuleSet`: one unsigned 32-bit field (the inline
    /// index) and two signed 32-bit fields.
    ContentRestrictionRuleSet,
    /// `DataTagInfo`: one string plus two signed 32-bit fields; the inline
    /// index is field 1.
    DataTagInfo,
    /// `AnimKitBoneSetAlias`: two unsigned 8-bit fields, separate u32 id table.
    AnimKitBoneSetAlias,
    /// `AnimKitPriority`: one unsigned 8-bit field, separate u32 id table.
    AnimKitPriority,
    /// `AnimReplacementSet`: one unsigned 8-bit field, separate u32 id table.
    AnimReplacementSet,
    /// `AnimKitBoneSet`: one string, three signed 8/32-bit scalars and a signed
    /// 32-bit field, separate u32 id table. Its leading `FT_STRING_NOT_LOCALIZED`
    /// field decodes as ordinary text.
    AnimKitBoneSet,
    /// `AnimationData`: one unsigned 16-bit field, a signed 8-bit field, a
    /// signed 16-bit field and a signed 32-bit array of two; separate u32 id
    /// table.
    AnimationData,
    /// `AnimKitConfig`: a single signed 32-bit field, separate u32 id table.
    AnimKitConfig,
    /// `ItemExtendedCost`: one unsigned 64-bit field followed by ten further
    /// scalar/array fields, separate u32 id table. First reviewed table with a
    /// 64-bit (`FT_LONG`) unsigned field.
    ItemExtendedCost,
    /// `TransmogOutfitEntry`: one unsigned 64-bit field, a string and seven
    /// further fields; the id is read from the inline field 2.
    TransmogOutfitEntry,
    /// `BankTab`: one unsigned 64-bit field, two 8-bit fields and six signed
    /// 32-bit fields, separate u32 id table.
    BankTab,
    /// `ChrSpecialization`: the first reviewed table whose parent index field
    /// lies INSIDE the file field count (`ParentIndexField 4 < FileFieldCount
    /// 13`), so the parent value overwrites the existing signed 8-bit column 4
    /// rather than an appended field. FileDataId 1343390, IndexField 3,
    /// layout 0xDAB4CA4B.
    Specialization,
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
            "ChrCustomizationCategory" => Some(Self::CustomizationCategory),
            "ChrCustItemGeoModify" => Some(Self::CustItemGeoModify),
            "ChrCustomizationGeoset" => Some(Self::CustomizationGeoset),
            "ChrCustomizationMaterial" => Some(Self::CustomizationMaterial),
            "ChrCustomizationBoneSet" => Some(Self::CustomizationBoneSet),
            "ChrCustomizationElement" => Some(Self::CustomizationElement),
            "ChrCustomizationDisplayInfo" => Some(Self::CustomizationDisplayInfo),
            "ChrCustomizationReq" => Some(Self::CustomizationReq),
            "Curve" => Some(Self::Curve),
            "ContentRestrictionRuleSet" => Some(Self::ContentRestrictionRuleSet),
            "DataTagInfo" => Some(Self::DataTagInfo),
            "AnimKitBoneSetAlias" => Some(Self::AnimKitBoneSetAlias),
            "AnimKitPriority" => Some(Self::AnimKitPriority),
            "AnimReplacementSet" => Some(Self::AnimReplacementSet),
            "AnimKitBoneSet" => Some(Self::AnimKitBoneSet),
            "AnimationData" => Some(Self::AnimationData),
            "AnimKitConfig" => Some(Self::AnimKitConfig),
            "ItemExtendedCost" => Some(Self::ItemExtendedCost),
            "TransmogOutfitEntry" => Some(Self::TransmogOutfitEntry),
            "BankTab" => Some(Self::BankTab),
            "ChrSpecialization" => Some(Self::Specialization),
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
            Self::CustomizationCategory => "ChrCustomizationCategory",
            Self::CustItemGeoModify => "ChrCustItemGeoModify",
            Self::CustomizationGeoset => "ChrCustomizationGeoset",
            Self::CustomizationMaterial => "ChrCustomizationMaterial",
            Self::CustomizationBoneSet => "ChrCustomizationBoneSet",
            Self::CustomizationElement => "ChrCustomizationElement",
            Self::CustomizationDisplayInfo => "ChrCustomizationDisplayInfo",
            Self::CustomizationReq => "ChrCustomizationReq",
            Self::Curve => "Curve",
            Self::ContentRestrictionRuleSet => "ContentRestrictionRuleSet",
            Self::DataTagInfo => "DataTagInfo",
            Self::AnimKitBoneSetAlias => "AnimKitBoneSetAlias",
            Self::AnimKitPriority => "AnimKitPriority",
            Self::AnimReplacementSet => "AnimReplacementSet",
            Self::AnimKitBoneSet => "AnimKitBoneSet",
            Self::AnimationData => "AnimationData",
            Self::AnimKitConfig => "AnimKitConfig",
            Self::ItemExtendedCost => "ItemExtendedCost",
            Self::TransmogOutfitEntry => "TransmogOutfitEntry",
            Self::BankTab => "BankTab",
            Self::Specialization => "ChrSpecialization",
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
            Self::CustomizationCategory => 3526439,
            Self::CustItemGeoModify => 3952714,
            Self::CustomizationGeoset => 3456171,
            Self::CustomizationMaterial => 3459652,
            Self::CustomizationBoneSet => 3460181,
            Self::CustomizationElement => 3512765,
            Self::CustomizationDisplayInfo => 3593032,
            Self::CustomizationReq => 3450453,
            Self::Curve => 892585,
            Self::ContentRestrictionRuleSet => 4281307,
            Self::DataTagInfo => 6652252,
            Self::AnimKitBoneSetAlias => 1266937,
            Self::AnimKitPriority => 1266540,
            Self::AnimReplacementSet => 1284817,
            Self::AnimKitBoneSet => 1375433,
            Self::AnimationData => 1375431,
            Self::AnimKitConfig => 1300872,
            Self::ItemExtendedCost => 801681,
            Self::TransmogOutfitEntry => 7320164,
            Self::BankTab => 5242411,
            Self::Specialization => 1343390,
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
        // Case (a) no parent (`parent == -1`): `total_field_count == field_count`.
        // Case (b) parent in the data block (`0 <= parent < fields.len()`): the
        //   parent is an existing decoded column, so there is no appended field
        //   and `total_field_count == field_count` (both == fields.len()).
        // Case (c) extra field at the end (`parent >= fields.len()`): the
        //   reviewed `Meta->FieldCount` is one past the columns, so
        //   `total_field_count + 1 == fields.len() + 1`, i.e. total == columns.
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
            // Transcribed from DB2Metadata.h::ChrCustomizationCategoryMeta:
            // FileDataId 3526439, IndexField -1, ParentIndexField -1,
            // FieldCount == FileFieldCount, LayoutHash 0x1605B68A. One string
            // and six signed 32-bit fields plus a float.
            Self::CustomizationCategory => Schema {
                hash: None,
                layout: 0x1605B68A,
                index: -1,
                parent: -1,
                fields: &[
                    String,
                    Signed(32, 1),
                    Signed(32, 1),
                    Signed(32, 1),
                    Signed(32, 1),
                    Signed(32, 1),
                    Signed(32, 1),
                    Float(1),
                ],
            },
            // Transcribed from DB2Metadata.h::ChrCustItemGeoModifyMeta:
            // FileDataId 3952714, IndexField -1, ParentIndexField -1,
            // FieldCount == FileFieldCount, LayoutHash 0x2986E1F3. One signed
            // byte plus two signed 32-bit fields.
            Self::CustItemGeoModify => Schema {
                hash: None,
                layout: 0x2986E1F3,
                index: -1,
                parent: -1,
                fields: &[Signed(8, 1), Signed(32, 1), Signed(32, 1)],
            },
            // Transcribed from DB2Metadata.h::ChrCustomizationGeosetMeta:
            // FileDataId 3456171, IndexField -1, ParentIndexField -1,
            // FieldCount == FileFieldCount, LayoutHash 0xBF55F2FB. One signed
            // byte plus two signed 32-bit fields.
            Self::CustomizationGeoset => Schema {
                hash: None,
                layout: 0xBF55F2FB,
                index: -1,
                parent: -1,
                fields: &[Signed(8, 1), Signed(32, 1), Signed(32, 1)],
            },
            // Transcribed from DB2Metadata.h::ChrCustomizationMaterialMeta:
            // FileDataId 3459652, IndexField -1, ParentIndexField -1,
            // FieldCount == FileFieldCount, LayoutHash 0xBE9767E9. Two signed
            // 32-bit fields.
            Self::CustomizationMaterial => Schema {
                hash: None,
                layout: 0xBE9767E9,
                index: -1,
                parent: -1,
                fields: &[Signed(32, 1), Signed(32, 1)],
            },
            // Transcribed from DB2Metadata.h::ChrCustomizationBoneSetMeta:
            // FileDataId 3460181, IndexField -1, ParentIndexField -1,
            // FieldCount == FileFieldCount, LayoutHash 0xCC2CFF6E. Two signed
            // 32-bit fields.
            Self::CustomizationBoneSet => Schema {
                hash: None,
                layout: 0xCC2CFF6E,
                index: -1,
                parent: -1,
                fields: &[Signed(32, 1), Signed(32, 1)],
            },
            // Transcribed from DB2Metadata.h::ChrCustomizationElementMeta:
            // FileDataId 3512765, IndexField -1, ParentIndexField -1,
            // FieldCount == FileFieldCount, LayoutHash 0x6483C37E. Thirteen
            // signed 32-bit fields.
            Self::CustomizationElement => Schema {
                hash: None,
                layout: 0x6483C37E,
                index: -1,
                parent: -1,
                fields: &[
                    Signed(32, 1),
                    Signed(32, 1),
                    Signed(32, 1),
                    Signed(32, 1),
                    Signed(32, 1),
                    Signed(32, 1),
                    Signed(32, 1),
                    Signed(32, 1),
                    Signed(32, 1),
                    Signed(32, 1),
                    Signed(32, 1),
                    Signed(32, 1),
                    Signed(32, 1),
                ],
            },
            // Transcribed from DB2Metadata.h::ChrCustomizationDisplayInfoMeta:
            // FileDataId 3593032, IndexField -1, ParentIndexField -1,
            // FieldCount == FileFieldCount, LayoutHash 0xB24CE54F. Two signed
            // 32-bit fields then three floats.
            Self::CustomizationDisplayInfo => Schema {
                hash: None,
                layout: 0xB24CE54F,
                index: -1,
                parent: -1,
                fields: &[Signed(32, 1), Signed(32, 1), Float(1), Float(1), Float(1)],
            },
            // Transcribed from DB2Metadata.h::ChrCustomizationReqMeta:
            // FileDataId 3450453, IndexField -1, ParentIndexField -1,
            // FieldCount == FileFieldCount, LayoutHash 0xCA154412. One string,
            // seven signed 32-bit fields and a signed 32-bit array of two.
            Self::CustomizationReq => Schema {
                hash: None,
                layout: 0xCA154412,
                index: -1,
                parent: -1,
                fields: &[
                    String,
                    Signed(32, 1),
                    Signed(32, 1),
                    Signed(32, 1),
                    Signed(32, 1),
                    Signed(32, 1),
                    Signed(32, 1),
                    Signed(32, 1),
                    Signed(32, 2),
                ],
            },
            // Transcribed from DB2Metadata.h::CurveMeta:
            // FileDataId 892585, IndexField 0, ParentIndexField -1,
            // FieldCount == FileFieldCount, LayoutHash 0xD362535A. The id is read
            // from the inline field 0 (Unsigned(32,1)); field 1 is an UNSIGNED
            // FT_BYTE and field 2 a signed 32-bit field.
            Self::Curve => Schema {
                hash: None,
                layout: 0xD362535A,
                index: 0,
                parent: -1,
                fields: &[Unsigned(32, 1), Unsigned(8, 1), Signed(32, 1)],
            },
            // Transcribed from DB2Metadata.h::ContentRestrictionRuleSetMeta:
            // FileDataId 4281307, IndexField 0, ParentIndexField -1,
            // FieldCount == FileFieldCount, LayoutHash 0x5774D75A. One unsigned
            // 32-bit field and two signed 32-bit fields; the id is read from the
            // inline field 0 (FT_INT IsSigned = false).
            Self::ContentRestrictionRuleSet => Schema {
                hash: None,
                layout: 0x5774D75A,
                index: 0,
                parent: -1,
                fields: &[Unsigned(32, 1), Signed(32, 1), Signed(32, 1)],
            },
            // Transcribed from DB2Metadata.h::DataTagInfoMeta:
            // FileDataId 6652252, IndexField 1, ParentIndexField -1,
            // FieldCount == FileFieldCount, LayoutHash 0xDB46F772. One string
            // plus two signed 32-bit fields; the id is read from the inline
            // field 1.
            Self::DataTagInfo => Schema {
                hash: None,
                layout: 0xDB46F772,
                index: 1,
                parent: -1,
                fields: &[String, Unsigned(32, 1), Signed(32, 1)],
            },
            // Transcribed from DB2Metadata.h::AnimKitBoneSetAliasMeta:
            // FileDataId 1266937, IndexField -1, ParentIndexField -1,
            // FieldCount == FileFieldCount, LayoutHash 0x19A6986C. Two unsigned
            // 8-bit fields.
            Self::AnimKitBoneSetAlias => Schema {
                hash: None,
                layout: 0x19A6986C,
                index: -1,
                parent: -1,
                fields: &[Unsigned(8, 1), Unsigned(8, 1)],
            },
            // Transcribed from DB2Metadata.h::AnimKitPriorityMeta:
            // FileDataId 1266540, IndexField -1, ParentIndexField -1,
            // FieldCount == FileFieldCount, LayoutHash 0xCCF889D8. One unsigned
            // 8-bit field.
            Self::AnimKitPriority => Schema {
                hash: None,
                layout: 0xCCF889D8,
                index: -1,
                parent: -1,
                fields: &[Unsigned(8, 1)],
            },
            // Transcribed from DB2Metadata.h::AnimReplacementSetMeta:
            // FileDataId 1284817, IndexField -1, ParentIndexField -1,
            // FieldCount == FileFieldCount, LayoutHash 0x7C047695. One unsigned
            // 8-bit field.
            Self::AnimReplacementSet => Schema {
                hash: None,
                layout: 0x7C047695,
                index: -1,
                parent: -1,
                fields: &[Unsigned(8, 1)],
            },
            // Transcribed from DB2Metadata.h::AnimKitBoneSetMeta:
            // FileDataId 1375433, IndexField -1, ParentIndexField -1,
            // FieldCount == FileFieldCount, LayoutHash 0x43E7736F. One string
            // (FT_STRING_NOT_LOCALIZED, decoded as ordinary text) plus three
            // signed scalar fields and a signed 32-bit field.
            Self::AnimKitBoneSet => Schema {
                hash: None,
                layout: 0x43E7736F,
                index: -1,
                parent: -1,
                fields: &[
                    String,
                    Signed(32, 1),
                    Signed(8, 1),
                    Signed(8, 1),
                    Signed(32, 1),
                ],
            },
            // Transcribed from DB2Metadata.h::AnimationDataMeta:
            // FileDataId 1375431, IndexField -1, ParentIndexField -1,
            // FieldCount == FileFieldCount, LayoutHash 0xBBF66A3C. One unsigned
            // 16-bit field, a signed 8-bit field, a signed 16-bit field and a
            // signed 32-bit array of two.
            Self::AnimationData => Schema {
                hash: None,
                layout: 0xBBF66A3C,
                index: -1,
                parent: -1,
                fields: &[Unsigned(16, 1), Signed(8, 1), Signed(16, 1), Signed(32, 2)],
            },
            // Transcribed from DB2Metadata.h::AnimKitConfigMeta:
            // FileDataId 1300872, IndexField -1, ParentIndexField -1,
            // FieldCount == FileFieldCount, LayoutHash 0x140718EF. A single
            // signed 32-bit field.
            Self::AnimKitConfig => Schema {
                hash: None,
                layout: 0x140718EF,
                index: -1,
                parent: -1,
                fields: &[Signed(32, 1)],
            },
            // Transcribed from DB2Metadata.h::ItemExtendedCostMeta:
            // FileDataId 801681, IndexField -1, ParentIndexField -1,
            // FieldCount == FileFieldCount, LayoutHash 0x22331DAB. The leading
            // field is an UNSIGNED FT_LONG (64-bit) read as 8 little-endian
            // bytes in compression 0; the 5-count arrays follow.
            Self::ItemExtendedCost => Schema {
                hash: None,
                layout: 0x22331DAB,
                index: -1,
                parent: -1,
                fields: &[
                    Unsigned(64, 1),
                    Unsigned(16, 1),
                    Signed(8, 1),
                    Signed(32, 1),
                    Unsigned(8, 1),
                    Signed(32, 1),
                    Unsigned(8, 1),
                    Signed(32, 5),
                    Unsigned(16, 5),
                    Unsigned(16, 5),
                    Unsigned(32, 5),
                ],
            },
            // Transcribed from DB2Metadata.h::TransmogOutfitEntryMeta:
            // FileDataId 7320164, IndexField 2, ParentIndexField -1,
            // FieldCount == FileFieldCount, LayoutHash 0x91D51304. The leading
            // field is an UNSIGNED FT_LONG (64-bit); the id is read from the
            // inline field 2 (Unsigned(32,1)).
            Self::TransmogOutfitEntry => Schema {
                hash: None,
                layout: 0x91D51304,
                index: 2,
                parent: -1,
                fields: &[
                    Unsigned(64, 1),
                    String,
                    Unsigned(32, 1),
                    Signed(32, 1),
                    Unsigned(8, 1),
                    Signed(32, 1),
                    Unsigned(8, 1),
                    Float(1),
                    Signed(32, 1),
                ],
            },
            // Transcribed from DB2Metadata.h::BankTabMeta: FileDataId 5242411,
            // IndexField -1, ParentIndexField -1, FieldCount == FileFieldCount,
            // LayoutHash 0x5B2F9B36. The leading field is an UNSIGNED FT_LONG
            // (64-bit); the remaining fields are one unsigned byte, one signed
            // byte and six signed 32-bit fields.
            Self::BankTab => Schema {
                hash: None,
                layout: 0x5B2F9B36,
                index: -1,
                parent: -1,
                fields: &[
                    Unsigned(64, 1),
                    Unsigned(8, 1),
                    Signed(8, 1),
                    Signed(32, 1),
                    Signed(32, 1),
                    Signed(32, 1),
                    Signed(32, 1),
                    Signed(32, 1),
                    Signed(32, 1),
                ],
            },
            // Transcribed from DB2Metadata.h::ChrSpecializationMeta: FileDataId
            // 1343390, IndexField 3, ParentIndexField 4, FieldCount ==
            // FileFieldCount == 13, layout 0xDAB4CA4B. Field 3 is the inline
            // (Unsigned(32,1)) id; field 4 is a SIGNED 8-bit column that is ALSO
            // the reviewed parent index, so the parent lookup overwrites column
            // 4 in the data block rather than an appended field.
            Self::Specialization => Schema {
                hash: None,
                layout: 0xDAB4CA4B,
                index: 3,
                parent: 4,
                fields: &[
                    String,
                    String,
                    String,
                    Unsigned(32, 1),
                    Signed(8, 1),
                    Signed(8, 1),
                    Signed(8, 1),
                    Signed(8, 1),
                    Signed(32, 1),
                    Signed(32, 1),
                    Signed(8, 1),
                    Signed(32, 1),
                    Signed(32, 2),
                ],
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
