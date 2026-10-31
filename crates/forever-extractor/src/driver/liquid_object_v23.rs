// SPDX-License-Identifier: GPL-3.0-or-later
// Authored by RustyCore Extractors, 2026-10-29.
//! Canonical evidence for complete build-70450 LiquidObject source facts.
use crate::{Error, plan::LIQUID_OBJECT_COMPLETE_SOURCE_CODEC_PROFILE};
use forever_casc::integrity;
use forever_db2::{
    availability::{self, SectionState},
    terrain_sources,
    terrain_sources::LiquidObjectFields,
};

use super::CompleteDb2Source;

pub(crate) const PATH: &str = "vmaps/liquid-object-complete-source-v1.bin";
const MAGIC: &[u8; 8] = b"RCLIQ01\0";
const VERSION: u32 = 1;
const FILE_DATA_ID: u32 = 1_308_058;
const MAX_SOURCE_BYTES: usize = 16 * 1024 * 1024;
const FACTS_DOMAIN: &[u8] = b"liquid-object-complete-source-facts-v1\0";
const CODEC_BYTES: usize = 112;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Evidence {
    source_bytes: u64,
    source_sha256: [u8; 32],
    direct_rows: u64,
    copy_records: u64,
    materialized_rows: u64,
    facts_sha256: [u8; 32],
}

impl Evidence {
    pub(crate) fn from_source(bytes: &[u8]) -> Result<Self, Error> {
        if bytes.is_empty() || bytes.len() > MAX_SOURCE_BYTES {
            return Err("product_liquid_object_source_limit".into());
        }
        let structural =
            availability::read_structure_70450_for_table(&CompleteDb2Source(bytes), "LiquidObject")
                .map_err(|_| Error::from("product_liquid_object_source"))?;
        if structural
            .sections
            .iter()
            .any(|section| section.state != SectionState::Admitted)
        {
            return Err("product_liquid_object_source_partial".into());
        }
        let source = terrain_sources::read_liquid_objects_70450(bytes)
            .map_err(|_| Error::from("product_liquid_object_source"))?;
        let direct_rows = u64::try_from(source.direct_records().len())
            .map_err(|_| Error::from("product_liquid_object_source_limit"))?;
        let copy_records = u64::try_from(source.copies().len())
            .map_err(|_| Error::from("product_liquid_object_source_limit"))?;
        let facts_bytes = FACTS_DOMAIN
            .len()
            .checked_add(24)
            .and_then(|bytes| bytes.checked_add(source.direct_records().len().checked_mul(6)?))
            .and_then(|bytes| bytes.checked_add(source.copies().len().checked_mul(8)?))
            .and_then(|bytes| bytes.checked_add(300_000usize.checked_mul(6)?))
            .ok_or("product_liquid_object_source_limit")?;
        let mut facts = Vec::new();
        facts
            .try_reserve_exact(facts_bytes)
            .map_err(|_| Error::from("product_allocation"))?;
        facts.extend_from_slice(FACTS_DOMAIN);
        facts.extend_from_slice(&direct_rows.to_le_bytes());
        for row in source.direct_records() {
            facts.extend_from_slice(&row.id.to_le_bytes());
            facts.extend_from_slice(&row.fields.liquid_type_id.to_le_bytes());
        }
        facts.extend_from_slice(&copy_records.to_le_bytes());
        for copy in source.copies() {
            facts.extend_from_slice(&copy.new_id.to_le_bytes());
            facts.extend_from_slice(&copy.source_id.to_le_bytes());
        }
        let materialized = source
            .materialize()
            .map_err(|_| Error::from("product_liquid_object_source"))?;
        let materialized_rows = u64::try_from(materialized.len())
            .map_err(|_| Error::from("product_liquid_object_source_limit"))?;
        facts.extend_from_slice(&materialized_rows.to_le_bytes());
        for (id, LiquidObjectFields { liquid_type_id }) in materialized {
            facts.extend_from_slice(&id.to_le_bytes());
            facts.extend_from_slice(&liquid_type_id.to_le_bytes());
        }
        Ok(Self {
            source_bytes: bytes.len() as u64,
            source_sha256: integrity::digest(bytes),
            direct_rows,
            copy_records,
            materialized_rows,
            facts_sha256: integrity::digest(&facts),
        })
    }

    pub(crate) fn matches_source(self, bytes: &[u8]) -> Result<bool, Error> {
        Ok(Self::from_source(bytes)? == self)
    }

    pub(crate) fn encode(self) -> Result<Vec<u8>, Error> {
        let mut bytes = Vec::new();
        bytes
            .try_reserve_exact(CODEC_BYTES)
            .map_err(|_| Error::from("product_allocation"))?;
        bytes.extend_from_slice(MAGIC);
        bytes.extend_from_slice(&VERSION.to_le_bytes());
        bytes.extend_from_slice(&FILE_DATA_ID.to_le_bytes());
        bytes.extend_from_slice(&self.source_bytes.to_le_bytes());
        bytes.extend_from_slice(&self.source_sha256);
        bytes.extend_from_slice(&self.direct_rows.to_le_bytes());
        bytes.extend_from_slice(&self.copy_records.to_le_bytes());
        bytes.extend_from_slice(&self.materialized_rows.to_le_bytes());
        bytes.extend_from_slice(&self.facts_sha256);
        if bytes.len() != CODEC_BYTES || LIQUID_OBJECT_COMPLETE_SOURCE_CODEC_PROFILE.is_empty() {
            return Err("product_liquid_object_evidence".into());
        }
        Ok(bytes)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn codec_identity_and_source_validation_are_fixed() {
        assert_eq!(PATH, "vmaps/liquid-object-complete-source-v1.bin");
        assert_eq!(CODEC_BYTES, 112);
        assert_eq!(MAGIC, b"RCLIQ01\0");
        assert!(Evidence::from_source(&[]).is_err());
        assert!(Evidence::from_source(b"not a DB2").is_err());
    }
}
