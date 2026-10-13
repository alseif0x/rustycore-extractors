// SPDX-License-Identifier: GPL-3.0-or-later
//! Complete raw camera acquisition, with source-backed DB2 discovery.
//! Independently authored from metadata/traversal facts at the pinned C++
//! commit. No M2 parsing, waypoint generation or runtime acceptance is claimed.
pub mod batch;
pub mod manifest;
pub mod options;
mod output;
#[cfg(test)]
mod tests;
pub mod verify;

use forever_db2::{CharacterTable, DecodedTable, Metadata};
use std::{collections::BTreeSet, io::Cursor};

pub const ORACLE: &str = "02245dcd245e7433e524577656177723d3e4992e";
pub const SOURCE_FILE_ID: u32 = 1_294_214;
pub const SOURCE_LAYOUT: u32 = 0x8BFD_752B;
pub const SOURCE_NAME: &str = "CinematicCamera.db2";
pub const FILE_ID_FIELD: usize = 3;
pub const DISCOVERY_POLICY: &str = "direct_rows_u32_field3_v1";
pub const MAX_SOURCE_BYTES: usize = 16 * 1024 * 1024;
pub const MAX_FILE_BYTES: usize = 16 * 1024 * 1024;
pub const MAX_FILES: usize = 4096;
pub const MAX_TOTAL_BYTES: usize = 256 * 1024 * 1024;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DiscoveryFacts {
    pub direct_records: u32,
    pub unique_file_ids: usize,
    pub file_id_set_sha256: String,
}

/// No Debug implementation: the discovered IDs belong to private artifacts.
pub struct CameraPlan {
    file_ids: BTreeSet<u32>,
    facts: DiscoveryFacts,
}
impl CameraPlan {
    pub fn file_ids(&self) -> &BTreeSet<u32> {
        &self.file_ids
    }
    pub fn facts(&self) -> &DiscoveryFacts {
        &self.facts
    }
}

/// `System.cpp:394-410` uses direct records, not copy-produced final rows.
/// Keep zero and deduplicate/order exactly as its std::set<uint32> does.
/// Return only after the decoder validates the entire file and copy table.
pub fn discover(bytes: &[u8]) -> Result<CameraPlan, &'static str> {
    if bytes.len() > MAX_SOURCE_BYTES {
        return Err("camera_policy_limit");
    }
    let metadata = Metadata::read(&mut Cursor::new(bytes)).map_err(|error| error.code())?;
    let table = CharacterTable::from_catalog("CinematicCamera")
        .map_err(|_| "camera_source_schema")?
        .ok_or("camera_source_schema")?;
    if metadata.header.layout_hash != SOURCE_LAYOUT {
        return Err("camera_source_schema");
    }
    let (_, file_ids) = DecodedTable::read_with_direct_u32_values(bytes, table, FILE_ID_FIELD)
        .map_err(|error| error.code())?;
    if file_ids.len() > MAX_FILES {
        return Err("camera_policy_limit");
    }
    let facts = DiscoveryFacts {
        direct_records: metadata.header.record_count,
        unique_file_ids: file_ids.len(),
        file_id_set_sha256: file_id_set_hash(&file_ids),
    };
    Ok(CameraPlan { file_ids, facts })
}

/// Little-endian u32 values in ascending order, without count or separators.
pub(super) fn file_id_set_hash(ids: &BTreeSet<u32>) -> String {
    let mut bytes = Vec::with_capacity(ids.len() * 4);
    for id in ids {
        bytes.extend_from_slice(&id.to_le_bytes());
    }
    crate::sha256::hex(&bytes)
}

pub fn file_name(id: u32) -> String {
    format!("FILE{id:08X}.xxx")
}
pub fn relative_path(id: u32) -> String {
    format!("cameras/{}", file_name(id))
}
pub fn source_path(locale: crate::Locale) -> String {
    format!("dbc/{}/{}", locale.name(), SOURCE_NAME)
}
