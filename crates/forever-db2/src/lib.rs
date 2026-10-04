// SPDX-License-Identifier: GPL-3.0-or-later
//! Read-only WDC5 metadata inspection plus a bounded regular-record decoder for
//! the two character tables (ChrClasses/ChrRaces). No CASC, no network and no
//! generic or full extraction: other tables, sparse/encrypted/parent-bearing and
//! multi-section variants are rejected. A successful decode is not format parity.
//!
//! Wire evidence: TrinityCore 02245dcd245e7433e524577656177723d3e4992e,
//! DB2FileLoader.h::DB2Header/DB2SectionHeader and LoadHeaders. This is an
//! independent bounded implementation, not an imported C++ loader.
use std::io::{self, Read};
mod records;
pub use records::{CharacterTable, DecodedTable, RecordError, Value};

pub const HEADER_SIZE: usize = 204;
pub const SECTION_SIZE: usize = 40;
/// Inspector policy, not a native format limit. Never allocate from raw counts.
pub const MAX_SECTIONS: u32 = 1024;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Error {
    Io,
    Truncated,
    UnsupportedSignature,
    UnsupportedVersion,
    ParentLookupCount,
    SectionLimit,
}

impl Error {
    pub fn code(self) -> &'static str {
        match self {
            Self::Io => "input_io",
            Self::Truncated => "metadata_truncated",
            Self::UnsupportedSignature => "unsupported_signature",
            Self::UnsupportedVersion => "unsupported_version",
            Self::ParentLookupCount => "invalid_parent_lookup_count",
            Self::SectionLimit => "section_policy_limit",
        }
    }

    pub fn exit_code(self) -> u8 {
        match self {
            Self::UnsupportedSignature | Self::UnsupportedVersion => 3,
            Self::SectionLimit => 6,
            _ => 4,
        }
    }
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.code())
    }
}
impl std::error::Error for Error {}

/// Does not expose the opaque Schema[128] bytes or any client records.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Header {
    pub record_count: u32,
    pub field_count: u32,
    pub record_size: u32,
    pub string_table_size: u32,
    pub table_hash: u32,
    pub layout_hash: u32,
    pub min_id: u32,
    pub max_id: u32,
    pub locale_mask: u32,
    pub flags: u16,
    /// Native signed header field; NOT DB2Meta::IndexField.
    pub index_field: i16,
    pub total_field_count: u32,
    pub packed_data_offset: u32,
    pub parent_lookup_count: u32,
    pub column_meta_size: u32,
    pub common_data_size: u32,
    pub pallet_data_size: u32,
    pub section_count: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Section {
    /// Nonzero TactId declaration only. Does not establish key availability,
    /// readability or encryption of every record; never contains key material.
    pub tact_declared: bool,
    pub file_offset: u32,
    pub record_count: u32,
    pub string_table_size: u32,
    pub catalog_data_offset: u32,
    pub id_table_size: u32,
    pub parent_lookup_data_size: u32,
    pub catalog_data_count: u32,
    pub copy_table_count: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Metadata {
    pub header: Header,
    pub sections: Vec<Section>,
}

fn word(bytes: &[u8], at: usize) -> u32 {
    u32::from_le_bytes(bytes[at..at + 4].try_into().unwrap())
}
fn half(bytes: &[u8], at: usize) -> u16 {
    u16::from_le_bytes(bytes[at..at + 2].try_into().unwrap())
}

impl Header {
    pub fn parse(bytes: &[u8; HEADER_SIZE]) -> Result<Self, Error> {
        if &bytes[..4] != b"WDC5" {
            return Err(Error::UnsupportedSignature);
        }
        if word(bytes, 4) != 5 {
            return Err(Error::UnsupportedVersion);
        }
        if word(bytes, 184) > 1 {
            return Err(Error::ParentLookupCount);
        }
        if word(bytes, 200) > MAX_SECTIONS {
            return Err(Error::SectionLimit);
        }
        Ok(Self {
            record_count: word(bytes, 136),
            field_count: word(bytes, 140),
            record_size: word(bytes, 144),
            string_table_size: word(bytes, 148),
            table_hash: word(bytes, 152),
            layout_hash: word(bytes, 156),
            min_id: word(bytes, 160),
            max_id: word(bytes, 164),
            locale_mask: word(bytes, 168),
            flags: half(bytes, 172),
            index_field: i16::from_le_bytes(bytes[174..176].try_into().unwrap()),
            total_field_count: word(bytes, 176),
            packed_data_offset: word(bytes, 180),
            parent_lookup_count: word(bytes, 184),
            column_meta_size: word(bytes, 188),
            common_data_size: word(bytes, 192),
            pallet_data_size: word(bytes, 196),
            section_count: word(bytes, 200),
        })
    }
}

impl Section {
    fn parse(bytes: &[u8; SECTION_SIZE]) -> Self {
        Self {
            tact_declared: bytes[..8].iter().any(|byte| *byte != 0),
            file_offset: word(bytes, 8),
            record_count: word(bytes, 12),
            string_table_size: word(bytes, 16),
            catalog_data_offset: word(bytes, 20),
            id_table_size: word(bytes, 24),
            parent_lookup_data_size: word(bytes, 28),
            catalog_data_count: word(bytes, 32),
            copy_table_count: word(bytes, 36),
        }
    }
}

fn read_exact(reader: &mut impl Read, buffer: &mut [u8]) -> Result<(), Error> {
    reader.read_exact(buffer).map_err(|error| {
        if error.kind() == io::ErrorKind::UnexpectedEof {
            Error::Truncated
        } else {
            Error::Io
        }
    })
}

impl Metadata {
    /// Reads only 204 + 40 * section_count bytes from the current position.
    /// Success means metadata readable, NOT file/body/schema/build compatible.
    pub fn read(reader: &mut impl Read) -> Result<Self, Error> {
        let mut bytes = [0; HEADER_SIZE];
        read_exact(reader, &mut bytes)?;
        let header = Header::parse(&bytes)?;
        let mut sections = Vec::with_capacity(header.section_count as usize);
        for _ in 0..header.section_count {
            let mut bytes = [0; SECTION_SIZE];
            read_exact(reader, &mut bytes)?;
            sections.push(Section::parse(&bytes));
        }
        Ok(Self { header, sections })
    }
}

#[cfg(test)]
mod tests;
