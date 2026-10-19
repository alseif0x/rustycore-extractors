// SPDX-License-Identifier: GPL-3.0-or-later
// Authored 2026-10-19: build-70450 selected-map collision geometry product.
//! Linux v22 publishes only selected-domain VMap geometry. The shared geometry
//! runner's callbacks are retained in bounded provisional memory; package files
//! are written only after the runner returns its all-or-error Completed owner.
//! Run remains the authoritative provisional/sealed package state machine.

use crate::{Error, driver::disk::Preflight, plan::DiskBudget};
use forever_casc::{
    collision::{
        Class, Status, geometry, run,
        world::{self, TableFacts},
    },
    integrity,
    package::{Component, Context, Run},
};
use forever_vmap::{RawM2, assembly, placement, vmo, wmo::raw};
use std::{collections::BTreeMap, str};

pub(crate) const COVERAGE_PATH: &str = "vmaps/selected-geometry-coverage-v1.bin";
pub(crate) const COVERAGE_MAGIC: &[u8; 8] = b"RCVGEO1\0";
pub(crate) const PROFILE: &str = "linux70450-selected-map-geometry-v1";
const COVERAGE_VERSION: u32 = 1;
const COVERAGE_FACTS: u32 = 0x3ff;
const COVERAGE_MAX_BYTES: usize = 128 * 1024 * 1024;
pub(crate) const COVERAGE_MAX_MAPS: usize = 4096;
pub(crate) const COVERAGE_MAX_ARTIFACTS: usize = 65_535;
const MAX_STAGED_MODEL_BYTES: usize = 384 * 1024 * 1024;
const MAX_STAGED_PLACEMENT_BYTES: usize = 128 * 1024 * 1024;
pub(crate) const MAX_PLACEMENT_RECORDS: usize = 1_000_000;
pub(crate) const MAX_ASSEMBLY_MEMORY_BYTES: usize = 512 * 1024 * 1024;
pub(crate) const MAX_ASSEMBLY_OUTPUT_BYTES: usize = 128 * 1024 * 1024;
pub(crate) const MAX_ASSEMBLED_DOMAIN_BYTES: u64 = 128 * 1024 * 1024;
const LIMIT: &str = "product_selected_geometry_limit";
const SCHEMA: &str = "product_selected_geometry_coverage_schema";
const MODEL_SCOPE_PROFILE: &str = "placement-only-v1";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ModelScope {
    PlacementOnly,
}
impl ModelScope {
    const fn as_str(self) -> &'static str {
        match self {
            Self::PlacementOnly => "placement_only",
        }
    }
    const fn profile(self) -> &'static str {
        match self {
            Self::PlacementOnly => MODEL_SCOPE_PROFILE,
        }
    }
}

#[cfg(test)]
mod tests;

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct ArtifactFact {
    pub path: String,
    pub bytes: u64,
    pub sha: [u8; 32],
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct SectionFact {
    pub ordinal: u32,
    pub records: u32,
    pub copies: u32,
    pub admitted: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct AvailableMapFact {
    pub file_data_id: u32,
    pub declared_bytes: u64,
    pub available_bytes: u64,
    pub full_ckey_verified: bool,
    pub encoded_key_authenticated: bool,
    pub declared_direct_rows: u64,
    pub declared_copy_records: u64,
    pub direct_rows: u64,
    pub copy_records: u64,
    pub materialized_rows: u64,
    pub sections: Vec<SectionFact>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct LiquidFact {
    pub file_data_id: u32,
    pub bytes: u64,
    pub magic: [u8; 4],
    pub direct_rows: u64,
    pub copy_records: u64,
    pub materialized_rows: u64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct DomainMapFact {
    pub id: u32,
    pub parent: Option<u32>,
    pub wdt_file_data_id: u32,
    pub depth: u32,
    pub is_parent: bool,
}

pub(crate) struct Coverage {
    pub context: Context,
    pub requested_map: u32,
    pub gameobject_scope: geometry::GameObjectScope,
    pub gameobject_file_data_id: u32,
    pub gameobject_block_reason: geometry::GameObjectBlockReason,
    pub gameobject_request_made: bool,
    pub gameobject_startup_reads: u64,
    pub model_scope: ModelScope,
    pub liquid_object_scope: geometry::LiquidObjectScope,
    pub liquid_object_request_made: bool,
    pub map: AvailableMapFact,
    pub liquids: [LiquidFact; 2],
    pub maps: Vec<DomainMapFact>,
    pub artifacts: Vec<ArtifactFact>,
}

fn derive_model_scope(
    completed: &geometry::Completed,
    used_models: &std::collections::BTreeMap<String, ()>,
    artifact_inventory: &BTreeMap<String, ArtifactFact>,
) -> Result<(ModelScope, u64), Error> {
    let startup = completed.world().startup().data();
    let temp_list = forever_vmap::gameobject_models::TempList::parse(
        startup.lists().temp_bytes(),
        forever_vmap::gameobject_models::Limits::default(),
    )
    .map_err(|_| Error::from(SCHEMA))?;
    let startup_reads = u64::try_from(startup.startup_facts().reads).map_err(|_| LIMIT)?;
    if startup.table().file_data_id() != 0
        || startup_reads != 0
        || startup.stats().direct_rows != 0
        || startup.stats().copy_records != 0
        || startup.stats().candidate_requests != 0
        || !startup.outcomes().is_empty()
        || startup.lists().stats().rows != 0
        || temp_list.rows().next().is_some()
    {
        return Err(SCHEMA.into());
    }

    // Coverage may name only model artifacts corresponding one-to-one with the
    // actual placement requests already recorded by this producer. The BTreeMap
    // order makes this a bounded, allocation-free comparison.
    let mut artifacts = artifact_inventory
        .keys()
        .filter(|path| path.ends_with(".vmo"));
    for name in used_models.keys() {
        let path = artifacts.next().ok_or(SCHEMA)?;
        let artifact_name = path
            .strip_prefix("vmaps/")
            .and_then(|path| path.strip_suffix(".vmo"))
            .filter(|name| !name.is_empty() && !name.contains('/'))
            .ok_or(SCHEMA)?;
        if artifact_name != name {
            return Err(SCHEMA.into());
        }
    }
    if artifacts.next().is_some() {
        return Err(SCHEMA.into());
    }
    Ok((ModelScope::PlacementOnly, startup_reads))
}

impl Coverage {
    fn from_completed(
        completed: &geometry::Completed,
        context: Context,
        requested_map: u32,
        gameobject_file_data_id: u32,
        used_models: &std::collections::BTreeMap<String, ()>,
        artifact_inventory: &BTreeMap<String, ArtifactFact>,
    ) -> Result<Self, Error> {
        let facts = completed.coverage();
        let (model_scope, gameobject_startup_reads) =
            derive_model_scope(completed, used_models, artifact_inventory)?;
        if facts.gameobject_scope != geometry::GameObjectScope::Excluded
            || facts.gameobject_file_data_id
                != forever_casc::collision::gameobjects::TABLE_FILE_DATA_ID
            || facts.gameobject_block_reason != geometry::GameObjectBlockReason::ExcludedByProfile
            || facts.gameobject_request_made
            || facts.liquid_object_scope != geometry::LiquidObjectScope::OutOfScope
            || facts.liquid_object_request_made
            || completed.profile() != geometry::PROFILE
            || completed.selection()
                != forever_casc::collision::plan::MapSelection::MapAndAncestors(requested_map)
        {
            return Err(SCHEMA.into());
        }
        let map = available_map_fact(completed.world().map())?;
        let source_liquids = completed.world().liquids();
        let liquids = [
            liquid_fact(&source_liquids[0])?,
            liquid_fact(&source_liquids[1])?,
        ];
        if liquids[0].file_data_id != world::MATERIAL_FILE_DATA_ID
            || liquids[1].file_data_id != world::TYPE_FILE_DATA_ID
        {
            return Err(SCHEMA.into());
        }
        let mut maps = Vec::new();
        maps.try_reserve_exact(completed.world().topology().selected_active().count())
            .map_err(|_| Error::from("product_allocation"))?;
        for entry in completed.world().topology().selected_active() {
            maps.push(DomainMapFact {
                id: entry.id(),
                parent: entry.parent_map_id(),
                wdt_file_data_id: entry.wdt_file_data_id(),
                depth: u32::try_from(entry.depth()).map_err(|_| LIMIT)?,
                is_parent: entry.is_parent(),
            });
        }
        maps.sort_unstable_by_key(|entry| entry.id);
        if maps.is_empty()
            || maps.len() > COVERAGE_MAX_MAPS
            || !maps.iter().any(|entry| entry.id == requested_map)
            || maps
                .windows(2)
                .any(|entries| entries[0].id == entries[1].id)
        {
            return Err(SCHEMA.into());
        }
        if artifact_inventory.len() > COVERAGE_MAX_ARTIFACTS {
            return Err(LIMIT.into());
        }
        let mut artifacts = Vec::new();
        artifacts
            .try_reserve_exact(artifact_inventory.len())
            .map_err(|_| Error::from("product_allocation"))?;
        // BTreeMap traversal is already the canonical bytewise path order.
        for fact in artifact_inventory.values() {
            if artifacts.len() == COVERAGE_MAX_ARTIFACTS {
                return Err(LIMIT.into());
            }
            artifacts.push(fact.clone());
        }
        Ok(Self {
            context,
            requested_map,
            gameobject_scope: facts.gameobject_scope,
            gameobject_file_data_id,
            gameobject_block_reason: facts.gameobject_block_reason,
            gameobject_request_made: facts.gameobject_request_made,
            gameobject_startup_reads,
            model_scope,
            liquid_object_scope: facts.liquid_object_scope,
            liquid_object_request_made: facts.liquid_object_request_made,
            map,
            liquids,
            maps,
            artifacts,
        })
    }

    pub(crate) fn encode(&self) -> Result<Vec<u8>, Error> {
        let mut writer = Writer::new(COVERAGE_MAX_BYTES);
        writer.put(COVERAGE_MAGIC)?;
        writer.u32(COVERAGE_VERSION)?;
        writer.text(&self.context.json())?;
        writer.u32(self.requested_map)?;
        writer.text(crate::plan::SELECTED_GEOMETRY_COVERAGE_PROFILE)?;
        writer.text(PROFILE)?;
        writer.text("available-map-active-ancestors-v1")?;
        writer.text(crate::plan::SELECTED_GEOMETRY_GAMEOBJECT_SCOPE)?;
        writer.text(self.model_scope.profile())?;
        writer.text("available-section-map-partial-scope-v1")?;
        writer.bit(false)?; // No complete Map-source claim.
        writer.text(self.gameobject_scope.as_str())?;
        writer.text(self.gameobject_block_reason.as_str())?;
        writer.bit(self.gameobject_request_made)?;
        writer.text(self.model_scope.as_str())?;
        writer.text("complete-LiquidMaterial-LiquidType-v1")?;
        writer.text(self.liquid_object_scope.as_str())?;
        writer.bit(self.liquid_object_request_made)?;
        writer.bit(false)?; // No complete global-map-coverage claim.
        writer.u32(self.gameobject_file_data_id)?;
        writer.u64(self.gameobject_startup_reads)?;
        // Declared facts: available-section Map, GameObjects excluded by policy,
        // no GODI request or startup reads, structurally derived placement-only models, complete
        // liquid tables, LiquidObject out of scope with no request, and no
        // global-map coverage claim.
        writer.u32(COVERAGE_FACTS)?;
        writer.u32(self.map.file_data_id)?;
        writer.u64(self.map.declared_bytes)?;
        writer.u64(self.map.available_bytes)?;
        writer.bit(self.map.full_ckey_verified)?;
        writer.bit(self.map.encoded_key_authenticated)?;
        for value in [
            self.map.declared_direct_rows,
            self.map.declared_copy_records,
            self.map.direct_rows,
            self.map.copy_records,
            self.map.materialized_rows,
        ] {
            writer.u64(value)?;
        }
        writer.u32(u32::try_from(self.map.sections.len()).map_err(|_| LIMIT)?)?;
        for section in &self.map.sections {
            writer.u32(section.ordinal)?;
            writer.u32(section.records)?;
            writer.u32(section.copies)?;
            writer.bit(section.admitted)?;
        }
        writer.u32(2)?;
        for liquid in self.liquids {
            writer.u32(liquid.file_data_id)?;
            writer.u64(liquid.bytes)?;
            writer.put(&liquid.magic)?;
            writer.u64(liquid.direct_rows)?;
            writer.u64(liquid.copy_records)?;
            writer.u64(liquid.materialized_rows)?;
        }
        writer.u32(u32::try_from(self.maps.len()).map_err(|_| LIMIT)?)?;
        for map in &self.maps {
            writer.u32(map.id)?;
            writer.bit(map.parent.is_some())?;
            writer.u32(map.parent.unwrap_or(0))?;
            writer.u32(map.wdt_file_data_id)?;
            writer.u32(map.depth)?;
            writer.bit(map.is_parent)?;
        }
        writer.u32(u32::try_from(self.artifacts.len()).map_err(|_| LIMIT)?)?;
        for artifact in &self.artifacts {
            writer.text(&artifact.path)?;
            writer.u64(artifact.bytes)?;
            writer.put(&artifact.sha)?;
        }
        Ok(writer.bytes)
    }

    pub(crate) fn parse(bytes: &[u8]) -> Result<Self, Error> {
        if bytes.len() > COVERAGE_MAX_BYTES {
            return Err(LIMIT.into());
        }
        let mut reader = Reader { bytes, at: 0 };
        if reader.take(8)? != COVERAGE_MAGIC || reader.u32()? != COVERAGE_VERSION {
            return Err(SCHEMA.into());
        }
        let context = Context::parse(reader.text()?)?;
        if context.build != 70450 || context.components != Component::Collision.bit() {
            return Err(SCHEMA.into());
        }
        let requested_map = reader.u32()?;
        if reader.text()? != crate::plan::SELECTED_GEOMETRY_COVERAGE_PROFILE
            || reader.text()? != PROFILE
            || reader.text()? != "available-map-active-ancestors-v1"
            || reader.text()? != crate::plan::SELECTED_GEOMETRY_GAMEOBJECT_SCOPE
        {
            return Err(SCHEMA.into());
        }
        let model_scope = match (reader.text()?, reader.text()?) {
            (MODEL_SCOPE_PROFILE, "available-section-map-partial-scope-v1") => {
                ModelScope::PlacementOnly
            }
            _ => return Err(SCHEMA.into()),
        };
        if reader.bit()? {
            return Err(SCHEMA.into());
        }
        let gameobject_scope = match reader.text()? {
            "excluded" => geometry::GameObjectScope::Excluded,
            _ => return Err(SCHEMA.into()),
        };
        let gameobject_block_reason = match reader.text()? {
            "excluded_by_profile" => geometry::GameObjectBlockReason::ExcludedByProfile,
            _ => return Err(SCHEMA.into()),
        };
        let gameobject_request_made = reader.bit()?;
        if gameobject_request_made
            || reader.text()? != model_scope.as_str()
            || reader.text()? != "complete-LiquidMaterial-LiquidType-v1"
        {
            return Err(SCHEMA.into());
        }
        let liquid_object_scope = match reader.text()? {
            "out_of_scope" => geometry::LiquidObjectScope::OutOfScope,
            _ => return Err(SCHEMA.into()),
        };
        let liquid_object_request_made = reader.bit()?;
        if liquid_object_request_made || reader.bit()? {
            return Err(SCHEMA.into());
        }
        let gameobject_file_data_id = reader.u32()?;
        let gameobject_startup_reads = reader.u64()?;
        if gameobject_file_data_id != forever_casc::collision::gameobjects::TABLE_FILE_DATA_ID {
            return Err(SCHEMA.into());
        }
        if gameobject_startup_reads != 0 {
            return Err(SCHEMA.into());
        }
        if reader.u32()? != COVERAGE_FACTS {
            return Err(SCHEMA.into());
        }
        let file_data_id = reader.u32()?;
        let declared_bytes = reader.u64()?;
        let available_bytes = reader.u64()?;
        let full_ckey_verified = reader.bit()?;
        let encoded_key_authenticated = reader.bit()?;
        let declared_direct_rows = reader.u64()?;
        let declared_copy_records = reader.u64()?;
        let direct_rows = reader.u64()?;
        let copy_records = reader.u64()?;
        let materialized_rows = reader.u64()?;
        if file_data_id != world::MAP_FILE_DATA_ID
            || declared_bytes < 204
            || available_bytes > declared_bytes
            || (full_ckey_verified && available_bytes != declared_bytes)
            || (!full_ckey_verified && !encoded_key_authenticated)
            || direct_rows > declared_direct_rows
            || copy_records > declared_copy_records
            || materialized_rows < direct_rows
            || materialized_rows > direct_rows.saturating_add(copy_records)
        {
            return Err(SCHEMA.into());
        }
        let section_count = reader.count(16)?;
        let mut sections = Vec::new();
        sections
            .try_reserve_exact(section_count)
            .map_err(|_| Error::from("product_allocation"))?;
        let mut previous = None;
        let mut section_rows = 0_u64;
        let mut section_copies = 0_u64;
        let mut admitted_rows = 0_u64;
        let mut admitted_copies = 0_u64;
        for _ in 0..section_count {
            let section = SectionFact {
                ordinal: reader.u32()?,
                records: reader.u32()?,
                copies: reader.u32()?,
                admitted: reader.bit()?,
            };
            if section.ordinal >= 16 || previous.is_some_and(|ordinal| ordinal >= section.ordinal) {
                return Err(SCHEMA.into());
            }
            previous = Some(section.ordinal);
            section_rows = section_rows
                .checked_add(u64::from(section.records))
                .ok_or(LIMIT)?;
            section_copies = section_copies
                .checked_add(u64::from(section.copies))
                .ok_or(LIMIT)?;
            if section.admitted {
                admitted_rows = admitted_rows
                    .checked_add(u64::from(section.records))
                    .ok_or(LIMIT)?;
                admitted_copies = admitted_copies
                    .checked_add(u64::from(section.copies))
                    .ok_or(LIMIT)?;
            }
            sections.push(section);
        }
        if section_rows != declared_direct_rows
            || section_copies != declared_copy_records
            || admitted_rows != direct_rows
            || admitted_copies != copy_records
        {
            return Err(SCHEMA.into());
        }
        let map = AvailableMapFact {
            file_data_id,
            declared_bytes,
            available_bytes,
            full_ckey_verified,
            encoded_key_authenticated,
            declared_direct_rows,
            declared_copy_records,
            direct_rows,
            copy_records,
            materialized_rows,
            sections,
        };
        if reader.u32()? != 2 {
            return Err(SCHEMA.into());
        }
        let mut liquids = [LiquidFact {
            file_data_id: 0,
            bytes: 0,
            magic: [0; 4],
            direct_rows: 0,
            copy_records: 0,
            materialized_rows: 0,
        }; 2];
        for liquid in &mut liquids {
            *liquid = LiquidFact {
                file_data_id: reader.u32()?,
                bytes: reader.u64()?,
                magic: reader.take(4)?.try_into().map_err(|_| SCHEMA)?,
                direct_rows: reader.u64()?,
                copy_records: reader.u64()?,
                materialized_rows: reader.u64()?,
            };
            if liquid.bytes == 0
                || liquid.direct_rows > 100_000
                || liquid.copy_records > 100_000
                || liquid.materialized_rows < liquid.direct_rows
                || liquid.materialized_rows > liquid.direct_rows.saturating_add(liquid.copy_records)
            {
                return Err(SCHEMA.into());
            }
        }
        if liquids[0].file_data_id != world::MATERIAL_FILE_DATA_ID
            || liquids[1].file_data_id != world::TYPE_FILE_DATA_ID
        {
            return Err(SCHEMA.into());
        }
        let map_count = reader.count(COVERAGE_MAX_MAPS)?;
        if map_count == 0 {
            return Err(SCHEMA.into());
        }
        let mut maps = Vec::new();
        maps.try_reserve_exact(map_count)
            .map_err(|_| Error::from("product_allocation"))?;
        let mut previous_id = None;
        for _ in 0..map_count {
            let id = reader.u32()?;
            let has_parent = reader.bit()?;
            let parent = reader.u32()?;
            let entry = DomainMapFact {
                id,
                parent: has_parent.then_some(parent),
                wdt_file_data_id: reader.u32()?,
                depth: reader.u32()?,
                is_parent: reader.bit()?,
            };
            if (!has_parent && parent != 0)
                || entry.wdt_file_data_id == 0
                || previous_id.is_some_and(|value| value >= id)
            {
                return Err(SCHEMA.into());
            }
            previous_id = Some(id);
            maps.push(entry);
        }
        if !maps.iter().any(|entry| entry.id == requested_map) {
            return Err(SCHEMA.into());
        }
        let artifact_count = reader.count(COVERAGE_MAX_ARTIFACTS)?;
        let mut artifacts = Vec::new();
        artifacts
            .try_reserve_exact(artifact_count)
            .map_err(|_| Error::from("product_allocation"))?;
        let mut previous_path: Option<String> = None;
        for _ in 0..artifact_count {
            let path = reader.text()?.to_owned();
            let artifact = ArtifactFact {
                path,
                bytes: reader.u64()?,
                sha: reader.take(32)?.try_into().map_err(|_| SCHEMA)?,
            };
            if artifact.path.is_empty()
                || artifact.path.len() > 512
                || !artifact.path.starts_with("vmaps/")
                || previous_path
                    .as_ref()
                    .is_some_and(|previous| previous >= &artifact.path)
            {
                return Err(SCHEMA.into());
            }
            previous_path = Some(artifact.path.clone());
            artifacts.push(artifact);
        }
        if reader.at != bytes.len() {
            return Err(SCHEMA.into());
        }
        let coverage = Self {
            context,
            requested_map,
            gameobject_scope,
            gameobject_file_data_id,
            gameobject_block_reason,
            gameobject_request_made,
            gameobject_startup_reads,
            model_scope,
            liquid_object_scope,
            liquid_object_request_made,
            map,
            liquids,
            maps,
            artifacts,
        };
        if coverage.encode()?.as_slice() != bytes {
            return Err(SCHEMA.into());
        }
        Ok(coverage)
    }
}

fn available_map_fact(
    facts: &forever_casc::collision::available_70374::TableFacts,
) -> Result<AvailableMapFact, Error> {
    let mut sections = Vec::new();
    sections
        .try_reserve_exact(facts.sections().count())
        .map_err(|_| Error::from("product_allocation"))?;
    for section in facts.sections() {
        sections.push(SectionFact {
            ordinal: section.ordinal,
            records: section.declared_records,
            copies: section.declared_copies,
            admitted: section.state == forever_db2::availability::SectionState::Admitted,
        });
    }
    Ok(AvailableMapFact {
        file_data_id: facts.file_data_id(),
        declared_bytes: facts.declared_bytes,
        available_bytes: facts.available_bytes,
        full_ckey_verified: facts.full_ckey_verified,
        encoded_key_authenticated: facts.encoded_key_authenticated,
        declared_direct_rows: facts.declared_direct_rows as u64,
        declared_copy_records: facts.declared_copy_records as u64,
        direct_rows: facts.direct_rows as u64,
        copy_records: facts.copy_records as u64,
        materialized_rows: facts.materialized_rows as u64,
        sections,
    })
}

fn liquid_fact(facts: &TableFacts) -> Result<LiquidFact, Error> {
    Ok(LiquidFact {
        file_data_id: facts.source().file_data_id(),
        bytes: facts.source().byte_count() as u64,
        magic: facts.source().first_magic().ok_or(SCHEMA)?,
        direct_rows: facts.direct_rows as u64,
        copy_records: facts.copy_records as u64,
        materialized_rows: facts.materialized_rows as u64,
    })
}

struct Writer {
    bytes: Vec<u8>,
    maximum: usize,
}
impl Writer {
    fn new(maximum: usize) -> Self {
        Self {
            bytes: Vec::new(),
            maximum,
        }
    }
    fn put(&mut self, value: &[u8]) -> Result<(), Error> {
        let required = self.bytes.len().checked_add(value.len()).ok_or(LIMIT)?;
        if required > self.maximum {
            return Err(LIMIT.into());
        }
        if required > self.bytes.capacity() {
            self.bytes
                .try_reserve_exact(required - self.bytes.len())
                .map_err(|_| Error::from("product_allocation"))?;
            if self.bytes.capacity() > self.maximum {
                return Err(LIMIT.into());
            }
        }
        self.bytes.extend_from_slice(value);
        Ok(())
    }
    fn u32(&mut self, value: u32) -> Result<(), Error> {
        self.put(&value.to_le_bytes())
    }
    fn u64(&mut self, value: u64) -> Result<(), Error> {
        self.put(&value.to_le_bytes())
    }
    fn bit(&mut self, value: bool) -> Result<(), Error> {
        self.put(&[u8::from(value)])
    }
    fn text(&mut self, value: &str) -> Result<(), Error> {
        if value.len() > 4096 {
            return Err(LIMIT.into());
        }
        self.u32(u32::try_from(value.len()).map_err(|_| LIMIT)?)?;
        self.put(value.as_bytes())
    }
}

struct Reader<'a> {
    bytes: &'a [u8],
    at: usize,
}
impl<'a> Reader<'a> {
    fn take(&mut self, count: usize) -> Result<&'a [u8], Error> {
        let end = self.at.checked_add(count).ok_or(SCHEMA)?;
        let value = self.bytes.get(self.at..end).ok_or(SCHEMA)?;
        self.at = end;
        Ok(value)
    }
    fn u32(&mut self) -> Result<u32, Error> {
        Ok(u32::from_le_bytes(
            self.take(4)?.try_into().map_err(|_| SCHEMA)?,
        ))
    }
    fn u64(&mut self) -> Result<u64, Error> {
        Ok(u64::from_le_bytes(
            self.take(8)?.try_into().map_err(|_| SCHEMA)?,
        ))
    }
    fn bit(&mut self) -> Result<bool, Error> {
        match self.take(1)?[0] {
            0 => Ok(false),
            1 => Ok(true),
            _ => Err(SCHEMA.into()),
        }
    }
    fn count(&mut self, maximum: usize) -> Result<usize, Error> {
        let count = usize::try_from(self.u32()?).map_err(|_| LIMIT)?;
        if count > maximum {
            return Err(LIMIT.into());
        }
        Ok(count)
    }
    fn text(&mut self) -> Result<&'a str, Error> {
        let length = self.count(4096)?;
        str::from_utf8(self.take(length)?).map_err(|_| SCHEMA.into())
    }
}

#[derive(Clone, Copy)]
struct ModelIdentity {
    class: Class,
    raw_sha: [u8; 32],
}
struct StagedModel {
    identity: ModelIdentity,
    bounds: Option<[u32; 6]>,
    bytes: Vec<u8>,
}
struct Sink<'a> {
    run: &'a mut Run,
    models: BTreeMap<String, StagedModel>,
    placements: BTreeMap<u32, Vec<u8>>,
    model_bytes: usize,
    model_limit: usize,
    placement_bytes: usize,
    placement_limit: usize,
    placement_capacity: usize,
    failure: Option<Error>,
}
impl Sink<'_> {
    fn reject(&mut self, error: Error) -> Result<(), run::SinkError> {
        if self.failure.is_none() {
            self.failure = Some(error);
        }
        Err(run::SinkError::Rejected)
    }
    fn stage_model(&mut self, model: &forever_casc::collision::Model) -> Result<(), Error> {
        if model.status() != Status::Collision {
            return Ok(());
        }
        let raw_bytes = model.raw().ok_or("product_selected_geometry_model")?;
        let name = str::from_utf8(model.output_basename())
            .map_err(|_| Error::from("product_selected_geometry_model"))?;
        if name.is_empty()
            || name.len() > placement::MAX_NAME_BYTES
            || name.contains(['/', '\\'])
            || !name.is_ascii()
        {
            return Err("product_selected_geometry_model".into());
        }
        let identity = ModelIdentity {
            class: model.class(),
            raw_sha: integrity::digest(raw_bytes),
        };
        if let Some(previous) = self.models.get(name) {
            if previous.identity.class == identity.class
                && previous.identity.raw_sha == identity.raw_sha
            {
                return Ok(());
            }
            return Err("product_selected_geometry_model_alias".into());
        }
        if self.models.len() >= 4096 {
            return Err(LIMIT.into());
        }
        let remaining = self
            .model_limit
            .checked_sub(self.model_bytes)
            .ok_or(LIMIT)?;
        let mut limits = vmo::Limits::default();
        limits.max_bytes = limits.max_bytes.min(remaining);
        let (converted, bounds) = match model.class() {
            Class::M2 => {
                let raw = RawM2::parse(raw_bytes, forever_vmap::Limits::default())
                    .map_err(|error| Error::from(error.code()))?;
                (vmo::convert_m2(&raw, limits), Some(raw.bounds_bits()))
            }
            Class::Wmo => {
                let raw = raw::RawWmo::parse(raw_bytes, raw::ConversionLimits::default())
                    .map_err(|error| Error::from(error.code()))?;
                (vmo::convert_wmo(&raw, limits), None)
            }
        };
        let converted = converted.map_err(|error| Error::from(error.code()))?;
        let bytes = converted.into_bytes();
        let next = self.model_bytes.checked_add(bytes.len()).ok_or(LIMIT)?;
        if next > self.model_limit || bytes.len() > remaining {
            return Err(LIMIT.into());
        }
        self.models.insert(
            name.to_owned(),
            StagedModel {
                identity,
                bounds,
                bytes,
            },
        );
        self.model_bytes = next;
        Ok(())
    }
    fn stage_placements(&mut self, context: run::Context, bytes: &[u8]) -> Result<(), Error> {
        let next_total = self.placement_bytes.checked_add(bytes.len()).ok_or(LIMIT)?;
        if next_total > self.placement_limit {
            return Err(LIMIT.into());
        }
        let target = self.placements.entry(context.map_id).or_default();
        let next_len = target.len().checked_add(bytes.len()).ok_or(LIMIT)?;
        if next_len > placement::MAX_BYTES {
            return Err(LIMIT.into());
        }
        if next_len > target.capacity() {
            let staged_peak = self.placement_capacity.checked_add(next_len).ok_or(LIMIT)?;
            if staged_peak > self.placement_limit {
                return Err(LIMIT.into());
            }
            let old_capacity = target.capacity();
            target
                .try_reserve_exact(bytes.len())
                .map_err(|_| Error::from("product_allocation"))?;
            if target.capacity() > placement::MAX_BYTES {
                return Err(LIMIT.into());
            }
            self.placement_capacity = self
                .placement_capacity
                .checked_sub(old_capacity)
                .and_then(|capacity| capacity.checked_add(target.capacity()))
                .ok_or(LIMIT)?;
            if self.placement_capacity > self.placement_limit {
                return Err(LIMIT.into());
            }
        }
        target.extend_from_slice(bytes);
        self.placement_bytes = next_total;
        Ok(())
    }
}
impl run::Sink for Sink<'_> {
    fn model(&mut self, model: &forever_casc::collision::Model) -> Result<(), run::SinkError> {
        match self.stage_model(model) {
            Ok(()) => Ok(()),
            Err(error) => self.reject(error),
        }
    }
    fn placements(&mut self, context: run::Context, bytes: &[u8]) -> Result<(), run::SinkError> {
        match self.stage_placements(context, bytes) {
            Ok(()) => Ok(()),
            Err(error) => self.reject(error),
        }
    }
}

fn as_usize(value: u64) -> Result<usize, Error> {
    usize::try_from(value).map_err(|_| Error::from(LIMIT))
}

fn geometry_limits(budget: DiskBudget) -> Result<geometry::Limits, Error> {
    let mut limits = geometry::Limits::default();
    let raw_budget = as_usize(budget.raw_bytes)?;
    let wdt_budget = as_usize(budget.wdt_bytes)?;
    let cache_budget = as_usize(budget.cache_bytes)?;
    let placement_budget = as_usize(budget.placement_bytes)?;
    limits.startup.collision.max_source_bytes =
        limits.startup.collision.max_source_bytes.min(raw_budget);
    limits.startup.collision.max_raw_bytes = limits.startup.collision.max_raw_bytes.min(raw_budget);
    limits.world.run.max_wdt_memory_bytes = limits.world.run.max_wdt_memory_bytes.min(wdt_budget);
    limits.world.run.max_cache_memory_bytes =
        limits.world.run.max_cache_memory_bytes.min(cache_budget);
    limits.world.run.max_output_bytes = limits.world.run.max_output_bytes.min(
        budget
            .raw_bytes
            .checked_add(budget.placement_bytes)
            .ok_or(LIMIT)?,
    );
    limits.world.run.placements.max_models = limits.world.run.placements.max_models.min(4096);
    if placement_budget == 0 || raw_budget == 0 {
        return Err(LIMIT.into());
    }
    Ok(limits)
}

fn map_paths(id: u32) -> (String, String) {
    (
        format!("vmaps/{id:04}/{id:04}.vmtree"),
        format!("vmaps/coverage/{id:04}.placements"),
    )
}

fn tile_path(map: u32, x: u32, y: u32, index: bool) -> String {
    format!(
        "vmaps/{map:04}/{map:04}_{x:02}_{y:02}.{}",
        if index { "vmtileidx" } else { "vmtile" }
    )
}

fn publish(
    run: &mut Run,
    preflight: &Preflight,
    facts: &mut BTreeMap<String, ArtifactFact>,
    path: String,
    bytes: &[u8],
) -> Result<(), Error> {
    let sha = integrity::digest(bytes);
    if let Some(previous) = facts.get(&path) {
        if previous.bytes != bytes.len() as u64 || previous.sha != sha {
            return Err("product_selected_geometry_artifact_alias".into());
        }
        return Ok(());
    }
    if facts.len() >= COVERAGE_MAX_ARTIFACTS {
        return Err(LIMIT.into());
    }
    preflight.verify_unchanged()?;
    run.put(Component::Collision, &path, bytes)?;
    preflight.verify_unchanged()?;
    facts.insert(
        path.clone(),
        ArtifactFact {
            path,
            bytes: bytes.len() as u64,
            sha,
        },
    );
    Ok(())
}

pub(crate) fn extract(
    plan: &crate::plan::Plan,
    store: &mut forever_casc::local_store::LocalStore,
    run: &mut Run,
    preflight: &Preflight,
    context: Context,
    budget: DiskBudget,
) -> Result<(), Error> {
    if !plan.is_v22()
        || context.build != 70450
        || context.components != Component::Collision.bit()
        || plan.map_domain.is_none()
    {
        return Err("product_plan_component_scope".into());
    }
    let requested_map = plan.map_domain.ok_or("product_plan_map_domain")?;
    preflight.verify_unchanged()?;
    let mut sink = Sink {
        run,
        models: BTreeMap::new(),
        placements: BTreeMap::new(),
        model_bytes: 0,
        model_limit: as_usize(budget.vmo_bytes)?.min(MAX_STAGED_MODEL_BYTES),
        placement_bytes: 0,
        placement_limit: as_usize(budget.placement_bytes)?.min(MAX_STAGED_PLACEMENT_BYTES),
        placement_capacity: 0,
        failure: None,
    };
    let completed =
        match geometry::extract_map(store, requested_map, &mut sink, geometry_limits(budget)?) {
            Ok(completed) => completed,
            Err(error) => {
                return Err(sink
                    .failure
                    .take()
                    .unwrap_or_else(|| Error::from(error.code())));
            }
        };
    let report = completed.report().run;
    if report.build != 70450
        || report.wdt_resident_bytes as u64 > budget.wdt_bytes
        || report.cache_peak_bytes as u64 > budget.cache_bytes
        || report.model_raw_bytes > budget.raw_bytes
        || report.placement_bytes > budget.placement_bytes
    {
        return Err(LIMIT.into());
    }

    let topology = completed.world().topology();
    let mut domain = Vec::new();
    let domain_count = topology.selected_active().count();
    domain
        .try_reserve_exact(domain_count)
        .map_err(|_| Error::from("product_allocation"))?;
    for entry in topology.selected_active() {
        domain.push(DomainMapFact {
            id: entry.id(),
            parent: entry.parent_map_id(),
            wdt_file_data_id: entry.wdt_file_data_id(),
            depth: u32::try_from(entry.depth()).map_err(|_| LIMIT)?,
            is_parent: entry.is_parent(),
        });
    }
    domain.sort_unstable_by_key(|entry| (entry.depth, entry.id));
    if domain.is_empty() || domain.len() > COVERAGE_MAX_MAPS {
        return Err(LIMIT.into());
    }
    let mut ids = Vec::new();
    ids.try_reserve_exact(domain.len())
        .map_err(|_| Error::from("product_allocation"))?;
    ids.extend(domain.iter().map(|entry| entry.id));
    ids.sort_unstable();
    if sink
        .placements
        .keys()
        .any(|id| ids.binary_search(id).is_err())
    {
        return Err("product_selected_geometry_domain".into());
    }

    let mut artifacts = BTreeMap::new();
    let mut model_total = 0_u64;
    let mut map_total = 0_u64;
    let map_total_limit = budget.map_bytes.min(MAX_ASSEMBLED_DOMAIN_BYTES);
    let mut parent_payloads = BTreeMap::<(u32, u32, u32), Vec<u8>>::new();
    let mut parent_payload_bytes = 0usize;
    let mut parent_payload_capacity = 0usize;
    let mut used_models = std::collections::BTreeMap::<String, ()>::new();
    let mut placement_limits = placement::Limits::default();
    placement_limits.max_bytes = placement_limits.max_bytes.min(placement::MAX_BYTES);
    placement_limits.max_records = placement_limits.max_records.min(MAX_PLACEMENT_RECORDS);
    for map in &domain {
        let empty = Vec::new();
        let bytes = sink.placements.get(&map.id).unwrap_or(&empty);
        let parsed = placement::DirBin::parse(bytes, placement_limits)
            .map_err(|error| Error::from(error.code()))?;
        if parsed.record_count() > MAX_PLACEMENT_RECORDS {
            return Err(LIMIT.into());
        }
        let mut records = Vec::new();
        records
            .try_reserve_exact(parsed.record_count())
            .map_err(|_| Error::from("product_allocation"))?;
        for record in parsed.records() {
            let name = str::from_utf8(record.name())
                .map_err(|_| Error::from("product_selected_geometry_model"))?;
            let model = sink
                .models
                .get(name)
                .ok_or("product_selected_geometry_model_binding")?;
            used_models.insert(name.to_owned(), ());
            records.push(
                placement::bounds::complete(record, model.bounds, placement_limits)
                    .map_err(|error| Error::from(error.code()))?,
            );
        }
        let mut limits = assembly::Limits::default();
        limits.max_memory_bytes = limits.max_memory_bytes.min(MAX_ASSEMBLY_MEMORY_BYTES);
        limits.max_input_bytes = limits.max_input_bytes.min(placement::MAX_BYTES);
        let remaining_map_bytes = as_usize(map_total_limit)?
            .checked_sub(usize::try_from(map_total).map_err(|_| LIMIT)?)
            .ok_or(LIMIT)?;
        let remaining_after_proof = remaining_map_bytes.checked_sub(bytes.len()).ok_or(LIMIT)?;
        limits.max_output_bytes = limits
            .max_output_bytes
            .min(MAX_ASSEMBLY_OUTPUT_BYTES)
            .min(remaining_after_proof);
        limits.max_spawns = limits.max_spawns.min(MAX_PLACEMENT_RECORDS);
        let assembled =
            assembly::build(map.id, &records, limits).map_err(|error| Error::from(error.code()))?;
        let (tree_path, proof_path) = map_paths(map.id);
        let proof = assembled.ordered_dir_bin();
        for tile in assembled.tiles() {
            let index = tile.index_bytes();
            if let Some(payload) = tile.payload() {
                assembly::verify_associations(payload, index, proof, limits)
                    .map_err(|error| Error::from(error.code()))?;
                if map.is_parent {
                    let next = parent_payload_bytes
                        .checked_add(payload.len())
                        .ok_or(LIMIT)?;
                    if next as u64 > map_total_limit {
                        return Err(LIMIT.into());
                    }
                    let mut retained = Vec::new();
                    retained
                        .try_reserve_exact(payload.len())
                        .map_err(|_| Error::from("product_allocation"))?;
                    if retained.capacity() > as_usize(map_total_limit)? {
                        return Err(LIMIT.into());
                    }
                    parent_payload_capacity = parent_payload_capacity
                        .checked_add(retained.capacity())
                        .ok_or(LIMIT)?;
                    if parent_payload_capacity > as_usize(map_total_limit)? {
                        return Err(LIMIT.into());
                    }
                    retained.extend_from_slice(payload);
                    if parent_payloads
                        .insert((map.id, tile.x(), tile.y()), retained)
                        .is_some()
                    {
                        return Err("product_selected_geometry_artifact_alias".into());
                    }
                    parent_payload_bytes = next;
                }
            } else {
                let mut ancestor = map.parent;
                let mut actual = None;
                while let Some(ancestor_id) = ancestor {
                    let ancestor_map = domain
                        .iter()
                        .find(|entry| entry.id == ancestor_id)
                        .ok_or("product_selected_geometry_domain")?;
                    if let Some(bytes) = parent_payloads.get(&(ancestor_id, tile.x(), tile.y())) {
                        actual = Some(bytes.as_slice());
                        break;
                    }
                    ancestor = ancestor_map.parent;
                }
                assembly::verify_inherited_associations(
                    actual.ok_or("product_selected_geometry_parent_payload")?,
                    index,
                    proof,
                    limits,
                )
                .map_err(|error| Error::from(error.code()))?;
            }
        }
        let mut size = u64::try_from(proof.len()).map_err(|_| LIMIT)?;
        size = size
            .checked_add(assembled.tree_bytes().len() as u64)
            .ok_or(LIMIT)?;
        for tile in assembled.tiles() {
            size = size
                .checked_add(tile.index_bytes().len() as u64)
                .and_then(|value| {
                    value.checked_add(tile.payload().map_or(0, |payload| payload.len()) as u64)
                })
                .ok_or(LIMIT)?;
        }
        map_total = map_total.checked_add(size).ok_or(LIMIT)?;
        if map_total > map_total_limit {
            return Err(LIMIT.into());
        }
        publish(sink.run, preflight, &mut artifacts, proof_path, proof)?;
        publish(
            sink.run,
            preflight,
            &mut artifacts,
            tree_path,
            assembled.tree_bytes(),
        )?;
        for tile in assembled.tiles() {
            publish(
                sink.run,
                preflight,
                &mut artifacts,
                tile_path(map.id, tile.x(), tile.y(), true),
                tile.index_bytes(),
            )?;
            if let Some(payload) = tile.payload() {
                publish(
                    sink.run,
                    preflight,
                    &mut artifacts,
                    tile_path(map.id, tile.x(), tile.y(), false),
                    payload,
                )?;
            }
        }
    }
    for name in used_models.keys() {
        let staged = sink
            .models
            .get(name)
            .ok_or("product_selected_geometry_model_binding")?;
        model_total = model_total
            .checked_add(staged.bytes.len() as u64)
            .ok_or(LIMIT)?;
        if model_total > budget.vmo_bytes {
            return Err(LIMIT.into());
        }
        publish(
            sink.run,
            preflight,
            &mut artifacts,
            format!("vmaps/{name}.vmo"),
            &staged.bytes,
        )?;
    }
    if artifacts.len() > COVERAGE_MAX_ARTIFACTS {
        return Err(LIMIT.into());
    }
    let coverage = Coverage::from_completed(
        &completed,
        context,
        requested_map,
        completed.coverage().gameobject_file_data_id,
        &used_models,
        &artifacts,
    )?;
    let bytes = coverage.encode()?;
    preflight.verify_unchanged()?;
    sink.run.put(Component::Collision, COVERAGE_PATH, &bytes)?;
    store
        .verify_unchanged()
        .map_err(|error| Error::from(error.code()))?;
    preflight.verify_unchanged()?;
    Ok(())
}
