// SPDX-License-Identifier: GPL-3.0-or-later
// Authored 2026-10-19: strict selected-map collision geometry profile.
//! Runs collision geometry for one available Build 70450 map and its active
//! ancestors. GODI startup is deliberately excluded; the returned coverage
//! declares that exclusion rather than implying complete GameObject coverage.
//! Under this versioned profile contract, the domain is derived from the
//! available-section Map reader (`prepare_available_target` /
//! `extract_selected_70450`) with active ancestor closure; an unavailable
//! ancestor is rejected with a typed error. LiquidMaterial and LiquidType are
//! read completely through the collision-family path. LiquidObject is outside
//! this collision-only profile.
//!
//! Every WMO/M2 request emitted by the selected domain's WDT/ADT path goes
//! through the existing strict shared runner. Missing or malformed inputs are
//! fatal. Sink callbacks are provisional; no Completed value is returned on
//! any error, and this module performs no publication or runtime operation.

use super::{Extraction, Model, available_70374, gameobjects, plan, run, target, world};
use crate::local_store::LocalStore;

pub const PROFILE: &str = available_70374::PROFILE_70450_SELECTED_GEOMETRY;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Limits {
    pub startup: gameobjects::Limits,
    pub world: world::Limits,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Error {
    Target(super::Error),
    Startup(gameobjects::Error),
    Available(available_70374::Error),
}
impl Error {
    pub fn code(self) -> &'static str {
        match self {
            Self::Target(e) => e.code(),
            Self::Startup(e) => e.code(),
            Self::Available(e) => e.code(),
        }
    }
}
impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.code())
    }
}
impl std::error::Error for Error {}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GameObjectScope {
    Excluded,
}
impl GameObjectScope {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Excluded => "excluded",
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GameObjectBlockReason {
    ExcludedByProfile,
}
impl GameObjectBlockReason {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::ExcludedByProfile => "excluded_by_profile",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LiquidObjectScope {
    OutOfScope,
}
impl LiquidObjectScope {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::OutOfScope => "out_of_scope",
        }
    }
}

/// Declared profile coverage, including the startup phase intentionally not
/// requested. Model scope belongs to the v22 producer, which derives it from
/// this executed startup and its own placement/artifact inventory. Since
/// `gameobject_request_made` is false, coverage cannot assert any observed
/// cause; only a real report linked to an execution that made the GODI request
/// can do so.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CoverageFacts {
    pub gameobject_scope: GameObjectScope,
    pub gameobject_file_data_id: u32,
    pub gameobject_block_reason: GameObjectBlockReason,
    pub gameobject_request_made: bool,
    pub liquid_object_scope: LiquidObjectScope,
    pub liquid_object_request_made: bool,
}

pub struct Completed {
    world: available_70374::Completed,
    coverage: CoverageFacts,
}
impl Completed {
    pub const fn profile(&self) -> &'static str {
        PROFILE
    }
    pub const fn world(&self) -> &available_70374::Completed {
        &self.world
    }
    pub const fn coverage(&self) -> &CoverageFacts {
        &self.coverage
    }
    pub const fn extraction(&self) -> &Extraction {
        self.world.startup().extraction()
    }
    pub const fn report(&self) -> &run::SharedReport {
        self.world.report()
    }
    pub fn model(&self, handle: super::ModelHandle) -> Option<&Model> {
        self.world
            .startup()
            .extraction()
            .models()
            .get(handle.index())
    }
    pub const fn selection(&self) -> plan::MapSelection {
        self.world.topology().selection()
    }
}

/// Extracts one admitted Build 70450 map plus its active ancestor closure.
/// The startup constructor does not request GODI, so callers cannot expect
/// startup GameObject rows. Domain derivation from the available-section Map
/// reader and complete LiquidMaterial/LiquidType validation occur before the
/// shared runner invokes any Sink callback. Map is not read completely; its
/// partial availability is declared by this profile's versioned contract.
pub fn extract_map(
    store: &mut LocalStore,
    map_id: u32,
    sink: &mut impl run::Sink,
    limits: Limits,
) -> Result<Completed, Error> {
    target::Target::Build70450
        .check(store)
        .map_err(Error::Target)?;
    let startup =
        gameobjects::extract_startup_without_godi(store, limits.startup).map_err(Error::Startup)?;
    let world = available_70374::extract_selected_70450(startup, map_id, sink, limits.world)
        .map_err(Error::Available)?;
    Ok(Completed {
        world,
        coverage: CoverageFacts {
            gameobject_scope: GameObjectScope::Excluded,
            gameobject_file_data_id: gameobjects::TABLE_FILE_DATA_ID,
            gameobject_block_reason: GameObjectBlockReason::ExcludedByProfile,
            gameobject_request_made: false,
            liquid_object_scope: LiquidObjectScope::OutOfScope,
            liquid_object_request_made: false,
        },
    })
}

#[cfg(test)]
mod tests;
