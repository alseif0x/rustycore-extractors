// SPDX-License-Identifier: GPL-3.0-or-later
//! Schema facts independently transcribed from DB2Metadata.h::{ChrClassesMeta,
//! ChrRacesMeta}, TrinityCore 02245dcd245e7433e524577656177723d3e4992e, lines
//! 3455 (43 fields) and 4182 (51 fields). No upstream code imported.
//!
//! The two leading ChrRaces fields are FT_STRING_NOT_LOCALIZED in the reference
//! (DB2Metadata.h:4182); this implementation decodes both as ordinary text, so
//! they map to the same `Kind::String` with no behavior difference.
#[derive(Debug, Clone, Copy)]
pub enum CharacterTable {
    Classes,
    Races,
}
impl CharacterTable {
    pub fn named(name: &str) -> Option<Self> {
        match name {
            "ChrClasses" => Some(Self::Classes),
            "ChrRaces" => Some(Self::Races),
            _ => None,
        }
    }
    pub fn name(self) -> &'static str {
        match self {
            Self::Classes => "ChrClasses",
            Self::Races => "ChrRaces",
        }
    }
    pub fn file_id(self) -> u32 {
        match self {
            Self::Classes => 1361031,
            Self::Races => 1305311,
        }
    }
    /// True only when the header's identity fields match the reviewed schema:
    /// table hash, layout hash, field count and total field count.
    ///
    /// The native header's `IndexField` is deliberately NOT part of this
    /// comparison. Reference `DB2FileLoader.cpp:1746` only endian-converts
    /// `_header.IndexField` and never consults it; whether the index lives in
    /// the record body vs. a separate id table is decided from the reviewed
    /// metadata via `Meta->HasIndexFieldInData()` (`DB2Meta.cpp:22-25`,
    /// `Meta->IndexField != -1`), i.e. this schema's own `index`, not the
    /// header word. Real 70205 evidence confirms the two disagree (ChrRaces
    /// header `IndexField == 0` while the reviewed meta index is `-1`).
    pub fn matches(self, header: &crate::Header) -> bool {
        let schema = self.schema();
        header.table_hash == schema.hash
            && header.layout_hash == schema.layout
            && header.field_count as usize == schema.fields.len()
            && header.total_field_count == header.field_count
    }
    /// Reviewed schema identity `(table_hash, layout_hash, index, field_count)`.
    /// The `index` element is the authoritative metadata index used for inline
    /// vs. id-table decoding; it is NOT a gate and does not describe the header
    /// word.
    pub fn schema_identity(self) -> (u32, u32, i16, usize) {
        let schema = self.schema();
        (
            schema.hash,
            schema.layout,
            schema.index,
            schema.fields.len(),
        )
    }
    pub(super) fn schema(self) -> Schema {
        use Kind::*;
        match self {
            Self::Classes => Schema {
                hash: 0xF5889D8C,
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
                hash: 0x53F1783C,
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
    pub hash: u32,
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
