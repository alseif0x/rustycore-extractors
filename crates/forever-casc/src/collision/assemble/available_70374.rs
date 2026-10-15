// SPDX-License-Identifier: GPL-3.0-or-later
// Authored 2026-10-15: explicit available-world resident collision assembly.
//! The constructor consumes an available startup and creates its own private
//! placement Stage. No caller placements/proofs or complete-source conversions
//! are accepted. Source coverage and authentication claims stay attached to the
//! distinct available world owner. All known dependencies remain strict.
//! Same resident stage/world/output budgets, byte pipeline, parent proof checks
//! and deterministic artifact order as the existing complete-source assembler.
//! No product publication, omission policy or runtime acceptance follows.
#![cfg(target_os = "linux")]
use super::{Artifact, Limits, PackageParts, Stage, Stats, WorldView, artifacts_for, models_for};
use crate::collision::available_70374 as available;
use forever_vmap::{assembly, vmo};

#[cfg(test)]
mod tests;

pub const PROFILE: &str = "linux70374-disabled-key-available-collision-assembly-v1";
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Error {
    Assembly(super::Error),
    World(available::Error),
}
impl Error {
    pub fn code(self) -> &'static str {
        match self {
            Self::Assembly(e) => e.code(),
            Self::World(e) => e.code(),
        }
    }
}
impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.code())
    }
}
impl std::error::Error for Error {}

/// Keeps available GODI/Map facts and complete dependencies alive with all
/// borrowed artifact names/dtree bytes and owned ordered map/model proofs.
pub struct Completed {
    world: available::Completed,
    parts: PackageParts,
}
impl Completed {
    pub const fn profile(&self) -> &'static str {
        PROFILE
    }
    pub const fn world(&self) -> &available::Completed {
        &self.world
    }
    pub fn maps(&self) -> &[assembly::AssembledMap] {
        &self.parts.maps
    }
    pub const fn stats(&self) -> Stats {
        self.parts.stats
    }
    pub fn models(&self) -> impl Iterator<Item = (&[u8], &vmo::Model)> {
        models_for(
            self.world.startup().extraction().models(),
            &self.parts.models,
        )
    }
    pub fn artifacts(&self) -> impl Iterator<Item = Artifact<'_>> {
        artifacts_for(
            self.world.startup().extraction().models(),
            self.world.startup().data().lists(),
            &self.parts.maps,
            &self.parts.models,
        )
    }
}
/// Acquire the remaining world and assemble together. Failure consumes startup
/// and returns no world, sealed stage, earlier model or successful package.
pub fn extract(mut startup: available::Startup<'_>, limits: Limits) -> Result<Completed, Error> {
    limits.validate().map_err(Error::Assembly)?;
    let ceiling = limits
        .max_memory_bytes
        .checked_sub(limits.max_stage_bytes)
        .ok_or(Error::Assembly(super::Error::Limit))?;
    let collector = startup.collector_mut();
    collector
        .constrain_memory(ceiling.min(collector.memory_limit()))
        .map_err(|e| Error::Assembly(super::Error::Collision(e)))?;
    let mut stage = Stage::new(limits);
    let result = available::extract(startup, &mut stage, limits.world);
    if let Some(error) = stage.error {
        return Err(Error::Assembly(error));
    }
    let world = result.map_err(Error::World)?;
    let parts = super::build_parts(
        WorldView {
            source: world.startup().extraction().models(),
            lists: world.startup().data().lists(),
            topology: world.topology().assembly_topology(),
            resident_bytes: world.resident_bytes(),
            store_resident_bytes: world.store_resident_bytes(),
        },
        stage,
        limits,
    )
    .map_err(Error::Assembly)?;
    Ok(Completed { world, parts })
}
