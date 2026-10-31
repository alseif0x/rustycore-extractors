// SPDX-License-Identifier: GPL-3.0-or-later
// Authored 2026-10-19: strict selected-map collision geometry profile.
// Modified by the RustyCore Extractors project, 2026-10-24.
//! Runs collision geometry for one available Build 70450 map and its active
//! ancestors, or every admitted active map under the separate all-available
//! profiles through v46. The v47 GameObjectExpanded profile executes GODI
//! startup explicitly and reports its available-FDID direct-row scope.
//! Under each versioned profile contract, the domain is derived from the
//! available-section Map reader with the corresponding explicit map selection;
//! an unavailable ancestor or active map is rejected with a typed error.
//! LiquidMaterial and LiquidType are read completely through the
//! collision-family path. LiquidObject is outside this collision-only profile.
//!
//! Every WMO/M2 request emitted by the selected domain's WDT/ADT path goes
//! through the existing strict shared runner. V47 first executes the established
//! available-FDID GODI startup and keeps those outcomes in the shared collector.
//! Missing or malformed inputs are fatal outside the opt-in profiles, which declare
//! only WDT or WMO-doodad M2
//! FDIDs whose raw ROOT has no rows. The v34 opt-in also declares strictly proven
//! intrinsically empty WMO models; v35 separately declares valid source geometry
//! skipped before selection; v36 declares non-skipped source geometry removed by
//! the production material filter. Sink callbacks are provisional; no Completed
//! value is returned on any error, and this module performs no publication or
//! runtime operation.

use super::{Extraction, Model, available_70374, gameobjects, plan, run, target, world};
use crate::local_store::LocalStore;

pub const PROFILE: &str = available_70374::PROFILE_70450_SELECTED_GEOMETRY;
pub const AVAILABLE_PROFILE: &str = available_70374::PROFILE_70450_AVAILABLE_GEOMETRY;
pub const AVAILABLE_ROOT_ABSENT_PROFILE: &str =
    available_70374::PROFILE_70450_AVAILABLE_ROOT_ABSENT_GEOMETRY;
pub const AVAILABLE_MODEL_ROOT_ABSENT_PROFILE: &str =
    available_70374::PROFILE_70450_AVAILABLE_MODEL_ROOT_ABSENT_GEOMETRY;
pub const AVAILABLE_MODEL_ROOT_ABSENT_REQUEST_CAPACITY_PROFILE: &str =
    available_70374::PROFILE_70450_AVAILABLE_MODEL_ROOT_ABSENT_REQUEST_CAPACITY_GEOMETRY;
pub const AVAILABLE_VACATED_WMO_PROFILE: &str =
    available_70374::PROFILE_70450_AVAILABLE_VACATED_WMO_GEOMETRY;
pub const AVAILABLE_EMPTY_WMO_PROFILE: &str =
    available_70374::PROFILE_70450_AVAILABLE_EMPTY_WMO_GEOMETRY;
pub const AVAILABLE_UNSELECTED_WMO_PROFILE: &str =
    available_70374::PROFILE_70450_AVAILABLE_UNSELECTED_WMO_GEOMETRY;
pub const AVAILABLE_MATERIAL_FILTERED_WMO_PROFILE: &str =
    available_70374::PROFILE_70450_AVAILABLE_MATERIAL_FILTERED_WMO_GEOMETRY;
pub const AVAILABLE_DEPENDENCY_HEAVY_MATERIAL_FILTERED_WMO_PROFILE: &str =
    available_70374::PROFILE_70450_AVAILABLE_DEPENDENCY_HEAVY_MATERIAL_FILTERED_WMO_GEOMETRY;
pub const AVAILABLE_SOURCE_HEAVY_MATERIAL_FILTERED_WMO_PROFILE: &str =
    available_70374::PROFILE_70450_AVAILABLE_SOURCE_HEAVY_MATERIAL_FILTERED_WMO_GEOMETRY;
pub const AVAILABLE_READ_HEAVY_MATERIAL_FILTERED_WMO_PROFILE: &str =
    available_70374::PROFILE_70450_AVAILABLE_READ_HEAVY_MATERIAL_FILTERED_WMO_GEOMETRY;
pub const AVAILABLE_MODEL_HEAVY_MATERIAL_FILTERED_WMO_PROFILE: &str =
    available_70374::PROFILE_70450_AVAILABLE_MODEL_HEAVY_MATERIAL_FILTERED_WMO_GEOMETRY;
pub const AVAILABLE_SOURCE_EXPANDED_MATERIAL_FILTERED_WMO_PROFILE: &str =
    available_70374::PROFILE_70450_AVAILABLE_SOURCE_EXPANDED_MATERIAL_FILTERED_WMO_GEOMETRY;
pub const AVAILABLE_MODEL_OUTPUT_EXPANDED_MATERIAL_FILTERED_WMO_PROFILE: &str =
    available_70374::PROFILE_70450_AVAILABLE_MODEL_OUTPUT_EXPANDED_MATERIAL_FILTERED_WMO_GEOMETRY;
pub const AVAILABLE_MODEL_BYTES_EXPANDED_MATERIAL_FILTERED_WMO_PROFILE: &str =
    available_70374::PROFILE_70450_AVAILABLE_MODEL_BYTES_EXPANDED_MATERIAL_FILTERED_WMO_GEOMETRY;
pub const AVAILABLE_SOURCE_EXPANDED_V2_MATERIAL_FILTERED_WMO_PROFILE: &str =
    available_70374::PROFILE_70450_AVAILABLE_SOURCE_EXPANDED_V2_MATERIAL_FILTERED_WMO_GEOMETRY;
pub const AVAILABLE_PLACEMENT_EXPANDED_MATERIAL_FILTERED_WMO_PROFILE: &str =
    available_70374::PROFILE_70450_AVAILABLE_PLACEMENT_EXPANDED_MATERIAL_FILTERED_WMO_GEOMETRY;
pub const AVAILABLE_ASSEMBLY_EXPANDED_MATERIAL_FILTERED_WMO_PROFILE: &str =
    available_70374::PROFILE_70450_AVAILABLE_ASSEMBLY_EXPANDED_MATERIAL_FILTERED_WMO_GEOMETRY;
pub const AVAILABLE_GAMEOBJECT_EXPANDED_MATERIAL_FILTERED_WMO_PROFILE: &str =
    available_70374::PROFILE_70450_AVAILABLE_GAMEOBJECT_EXPANDED_MATERIAL_FILTERED_WMO_GEOMETRY;
pub const AVAILABLE_GAMEOBJECT_MISSING_ROOT_MATERIAL_FILTERED_WMO_PROFILE: &str =
    available_70374::PROFILE_70450_AVAILABLE_GAMEOBJECT_MISSING_ROOT_MATERIAL_FILTERED_WMO_GEOMETRY;

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
    AvailableFdidDirectV1,
    AvailableFdidDirectMissingRootV1,
    AvailableFdidDirectRootAbsentOrEncryptedUnavailableV1,
}
impl GameObjectScope {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Excluded => "excluded",
            Self::AvailableFdidDirectV1 => "available-fdid-direct-v1",
            Self::AvailableFdidDirectMissingRootV1 => "available-fdid-direct-missing-root-v1",
            Self::AvailableFdidDirectRootAbsentOrEncryptedUnavailableV1 => {
                "available-fdid-direct-root-absent-or-encrypted-unavailable-v1"
            }
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GameObjectBlockReason {
    ExcludedByProfile,
    None,
}
impl GameObjectBlockReason {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::ExcludedByProfile => "excluded_by_profile",
            Self::None => "none",
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

/// Declared profile coverage includes whether GODI was actually requested.
/// Excluded profiles report their policy reason; v47 exposes executed startup
/// facts and keeps model-scope interpretation explicit.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CoverageFacts {
    pub gameobject_scope: GameObjectScope,
    pub gameobject_file_data_id: u32,
    pub gameobject_block_reason: GameObjectBlockReason,
    pub gameobject_request_made: bool,
    pub gameobject_startup_reads: u64,
    pub gameobject_sections: u64,
    pub gameobject_direct_rows: u64,
    pub gameobject_copy_records: u64,
    pub gameobject_outcomes: u64,
    pub model_scope: &'static str,
    pub liquid_object_scope: LiquidObjectScope,
    pub liquid_object_request_made: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GameObjectFacts {
    pub declared_bytes: u64,
    pub available_bytes: u64,
    pub full_ckey_verified: bool,
    pub encoded_key_authenticated: bool,
    pub declared_direct_rows: u64,
    pub declared_copy_records: u64,
    pub direct_rows: u64,
    pub copy_records: u64,
    pub materialized_rows: u64,
    pub candidate_requests: u64,
    pub direct_outcomes: u64,
    pub zero_fdids: u64,
    pub textures: u64,
    pub m2_no_collision: u64,
    pub wmo_doodad_only: u64,
    pub missing_root_rows: u64,
    pub missing_root_fdids: Vec<u32>,
    pub encrypted_unavailable_rows: u64,
    pub encrypted_unavailable_fdids: Vec<u32>,
    pub collision_rows: u64,
    pub list_rows: u64,
    pub list_m2_rows: u64,
    pub list_wmo_rows: u64,
    pub list_vertex_visits: u64,
    pub list_raw_visit_bytes: u64,
    pub temp_bytes: u64,
    pub dtree_bytes: u64,
    pub temp_sha256: [u8; 32],
    pub dtree_sha256: [u8; 32],
    pub work: crate::local_store::availability::Work,
    pub sections: Vec<available_70374::SectionFact>,
    pub outcomes: Vec<available_70374::GameObjectOutcomeFact>,
}

pub struct Completed {
    world: available_70374::Completed,
    coverage: CoverageFacts,
    gameobject_facts: Option<GameObjectFacts>,
}
impl Completed {
    pub const fn profile(&self) -> &'static str {
        self.world.profile()
    }
    pub const fn world(&self) -> &available_70374::Completed {
        &self.world
    }
    pub const fn gameobjects(&self) -> &available_70374::Data {
        self.world.startup().data()
    }
    pub const fn coverage(&self) -> &CoverageFacts {
        &self.coverage
    }
    pub const fn gameobject_facts(&self) -> Option<&GameObjectFacts> {
        self.gameobject_facts.as_ref()
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
        coverage: excluded_coverage(),
        gameobject_facts: None,
    })
}

fn excluded_coverage() -> CoverageFacts {
    CoverageFacts {
        gameobject_scope: GameObjectScope::Excluded,
        gameobject_file_data_id: gameobjects::TABLE_FILE_DATA_ID,
        gameobject_block_reason: GameObjectBlockReason::ExcludedByProfile,
        gameobject_request_made: false,
        gameobject_startup_reads: 0,
        gameobject_sections: 0,
        gameobject_direct_rows: 0,
        gameobject_copy_records: 0,
        gameobject_outcomes: 0,
        model_scope: "placement-only-v1",
        liquid_object_scope: LiquidObjectScope::OutOfScope,
        liquid_object_request_made: false,
    }
}

/// Extracts every admitted active map, including maps with no collision models.
/// Unknown Map sections and broader client coverage are not inferred from them.
pub fn extract_all(
    store: &mut LocalStore,
    sink: &mut impl run::Sink,
    limits: Limits,
) -> Result<Completed, Error> {
    target::Target::Build70450
        .check(store)
        .map_err(Error::Target)?;
    let startup =
        gameobjects::extract_startup_without_godi(store, limits.startup).map_err(Error::Startup)?;
    let world = available_70374::extract_all_70450(startup, sink, limits.world)
        .map_err(Error::Available)?;
    Ok(Completed {
        world,
        coverage: excluded_coverage(),
        gameobject_facts: None,
    })
}

pub fn extract_all_with_root_absence(
    store: &mut LocalStore,
    sink: &mut impl run::Sink,
    limits: Limits,
) -> Result<Completed, Error> {
    target::Target::Build70450
        .check(store)
        .map_err(Error::Target)?;
    let startup =
        gameobjects::extract_startup_without_godi(store, limits.startup).map_err(Error::Startup)?;
    let world = available_70374::extract_all_with_root_absence_70450(startup, sink, limits.world)
        .map_err(Error::Available)?;
    Ok(Completed {
        world,
        coverage: excluded_coverage(),
        gameobject_facts: None,
    })
}

pub fn extract_all_with_model_root_absence(
    store: &mut LocalStore,
    sink: &mut impl run::Sink,
    limits: Limits,
) -> Result<Completed, Error> {
    target::Target::Build70450
        .check(store)
        .map_err(Error::Target)?;
    let startup =
        gameobjects::extract_startup_without_godi(store, limits.startup).map_err(Error::Startup)?;
    let world =
        available_70374::extract_all_with_model_root_absence_70450(startup, sink, limits.world)
            .map_err(Error::Available)?;
    Ok(Completed {
        world,
        coverage: excluded_coverage(),
        gameobject_facts: None,
    })
}

pub fn extract_all_with_model_root_absence_request_capacity(
    store: &mut LocalStore,
    sink: &mut impl run::Sink,
    limits: Limits,
) -> Result<Completed, Error> {
    target::Target::Build70450
        .check(store)
        .map_err(Error::Target)?;
    let startup = gameobjects::extract_startup_without_godi_with_resources(
        store,
        limits.startup,
        super::resources::Profile::RequestHeavyV1,
    )
    .map_err(Error::Startup)?;
    let world = available_70374::extract_all_with_model_root_absence_request_capacity_70450(
        startup,
        sink,
        limits.world,
    )
    .map_err(Error::Available)?;
    Ok(Completed {
        world,
        coverage: excluded_coverage(),
        gameobject_facts: None,
    })
}

pub fn extract_all_with_vacated_wmo(
    store: &mut LocalStore,
    sink: &mut impl run::Sink,
    limits: Limits,
) -> Result<Completed, Error> {
    target::Target::Build70450
        .check(store)
        .map_err(Error::Target)?;
    let startup = gameobjects::extract_startup_without_godi_with_resources(
        store,
        limits.startup,
        super::resources::Profile::RequestHeavyV1,
    )
    .map_err(Error::Startup)?;
    let world = available_70374::extract_all_with_vacated_wmo_70450(startup, sink, limits.world)
        .map_err(Error::Available)?;
    Ok(Completed {
        world,
        coverage: excluded_coverage(),
        gameobject_facts: None,
    })
}

pub fn extract_all_with_empty_wmo(
    store: &mut LocalStore,
    sink: &mut impl run::Sink,
    limits: Limits,
) -> Result<Completed, Error> {
    target::Target::Build70450
        .check(store)
        .map_err(Error::Target)?;
    let startup = gameobjects::extract_startup_without_godi_with_resources(
        store,
        limits.startup,
        super::resources::Profile::RequestHeavyV1,
    )
    .map_err(Error::Startup)?;
    let world = available_70374::extract_all_with_empty_wmo_70450(startup, sink, limits.world)
        .map_err(Error::Available)?;
    Ok(Completed {
        world,
        coverage: excluded_coverage(),
        gameobject_facts: None,
    })
}

pub fn extract_all_with_unselected_wmo(
    store: &mut LocalStore,
    sink: &mut impl run::Sink,
    limits: Limits,
) -> Result<Completed, Error> {
    target::Target::Build70450
        .check(store)
        .map_err(Error::Target)?;
    let startup = gameobjects::extract_startup_without_godi_with_resources(
        store,
        limits.startup,
        super::resources::Profile::RequestHeavyV1,
    )
    .map_err(Error::Startup)?;
    let world = available_70374::extract_all_with_unselected_wmo_70450(startup, sink, limits.world)
        .map_err(Error::Available)?;
    Ok(Completed {
        world,
        coverage: excluded_coverage(),
        gameobject_facts: None,
    })
}

#[cfg(target_os = "linux")]
pub fn extract_all_with_material_filtered_wmo(
    store: &mut LocalStore,
    sink: &mut impl run::Sink,
    limits: Limits,
) -> Result<Completed, Error> {
    target::Target::Build70450
        .check(store)
        .map_err(Error::Target)?;
    let startup = gameobjects::extract_startup_without_godi_with_resources(
        store,
        limits.startup,
        super::resources::Profile::RequestHeavyV1,
    )
    .map_err(Error::Startup)?;
    let world =
        available_70374::extract_all_with_material_filtered_wmo_70450(startup, sink, limits.world)
            .map_err(Error::Available)?;
    Ok(Completed {
        world,
        coverage: excluded_coverage(),
        gameobject_facts: None,
    })
}

#[cfg(target_os = "linux")]
pub fn extract_all_with_dependency_heavy_material_filtered_wmo(
    store: &mut LocalStore,
    sink: &mut impl run::Sink,
    limits: Limits,
) -> Result<Completed, Error> {
    target::Target::Build70450
        .check(store)
        .map_err(Error::Target)?;
    let startup = gameobjects::extract_startup_without_godi_with_resources(
        store,
        limits.startup,
        super::resources::Profile::DependencyHeavyV1,
    )
    .map_err(Error::Startup)?;
    let world = available_70374::extract_all_with_dependency_heavy_material_filtered_wmo_70450(
        startup,
        sink,
        limits.world,
    )
    .map_err(Error::Available)?;
    Ok(Completed {
        world,
        coverage: excluded_coverage(),
        gameobject_facts: None,
    })
}

#[cfg(target_os = "linux")]
pub fn extract_all_with_source_heavy_material_filtered_wmo(
    store: &mut LocalStore,
    sink: &mut impl run::Sink,
    limits: Limits,
) -> Result<Completed, Error> {
    target::Target::Build70450
        .check(store)
        .map_err(Error::Target)?;
    let startup = gameobjects::extract_startup_without_godi_with_resources(
        store,
        limits.startup,
        super::resources::Profile::SourceHeavyV1,
    )
    .map_err(Error::Startup)?;
    let world = available_70374::extract_all_with_source_heavy_material_filtered_wmo_70450(
        startup,
        sink,
        limits.world,
    )
    .map_err(Error::Available)?;
    Ok(Completed {
        world,
        coverage: excluded_coverage(),
        gameobject_facts: None,
    })
}

#[cfg(target_os = "linux")]
pub fn extract_all_with_source_heavy_material_filtered_wmo_70450(
    store: &mut LocalStore,
    sink: &mut impl run::Sink,
    limits: Limits,
) -> Result<Completed, Error> {
    extract_all_with_source_heavy_material_filtered_wmo(store, sink, limits)
}

#[cfg(target_os = "linux")]
pub fn extract_all_with_read_heavy_material_filtered_wmo(
    store: &mut LocalStore,
    sink: &mut impl run::Sink,
    limits: Limits,
) -> Result<Completed, Error> {
    target::Target::Build70450
        .check(store)
        .map_err(Error::Target)?;
    let startup = gameobjects::extract_startup_without_godi_with_resources(
        store,
        limits.startup,
        super::resources::Profile::ReadHeavyV1,
    )
    .map_err(Error::Startup)?;
    let world = available_70374::extract_all_with_read_heavy_material_filtered_wmo_70450(
        startup,
        sink,
        limits.world,
    )
    .map_err(Error::Available)?;
    Ok(Completed {
        world,
        coverage: excluded_coverage(),
        gameobject_facts: None,
    })
}

#[cfg(target_os = "linux")]
pub fn extract_all_with_read_heavy_material_filtered_wmo_70450(
    store: &mut LocalStore,
    sink: &mut impl run::Sink,
    limits: Limits,
) -> Result<Completed, Error> {
    extract_all_with_read_heavy_material_filtered_wmo(store, sink, limits)
}

#[cfg(target_os = "linux")]
pub fn extract_all_with_model_heavy_material_filtered_wmo(
    store: &mut LocalStore,
    sink: &mut impl run::Sink,
    limits: Limits,
) -> Result<Completed, Error> {
    target::Target::Build70450
        .check(store)
        .map_err(Error::Target)?;
    let startup = gameobjects::extract_startup_without_godi_with_resources(
        store,
        limits.startup,
        super::resources::Profile::ModelHeavyV1,
    )
    .map_err(Error::Startup)?;
    let world = available_70374::extract_all_with_model_heavy_material_filtered_wmo_70450(
        startup,
        sink,
        limits.world,
    )
    .map_err(Error::Available)?;
    Ok(Completed {
        world,
        coverage: excluded_coverage(),
        gameobject_facts: None,
    })
}

#[cfg(target_os = "linux")]
pub fn extract_all_with_model_heavy_material_filtered_wmo_70450(
    store: &mut LocalStore,
    sink: &mut impl run::Sink,
    limits: Limits,
) -> Result<Completed, Error> {
    extract_all_with_model_heavy_material_filtered_wmo(store, sink, limits)
}

#[cfg(target_os = "linux")]
pub fn extract_all_with_source_expanded_material_filtered_wmo(
    store: &mut LocalStore,
    sink: &mut impl run::Sink,
    limits: Limits,
) -> Result<Completed, Error> {
    target::Target::Build70450
        .check(store)
        .map_err(Error::Target)?;
    let startup = gameobjects::extract_startup_without_godi_with_resources(
        store,
        limits.startup,
        super::resources::Profile::SourceExpandedV1,
    )
    .map_err(Error::Startup)?;
    let world = available_70374::extract_all_with_source_expanded_material_filtered_wmo_70450(
        startup,
        sink,
        limits.world,
    )
    .map_err(Error::Available)?;
    Ok(Completed {
        world,
        coverage: excluded_coverage(),
        gameobject_facts: None,
    })
}

#[cfg(target_os = "linux")]
pub fn extract_all_with_source_expanded_material_filtered_wmo_70450(
    store: &mut LocalStore,
    sink: &mut impl run::Sink,
    limits: Limits,
) -> Result<Completed, Error> {
    extract_all_with_source_expanded_material_filtered_wmo(store, sink, limits)
}

#[cfg(target_os = "linux")]
pub fn extract_all_with_source_expanded_v2_material_filtered_wmo(
    store: &mut LocalStore,
    sink: &mut impl run::Sink,
    limits: Limits,
) -> Result<Completed, Error> {
    target::Target::Build70450
        .check(store)
        .map_err(Error::Target)?;
    let startup = gameobjects::extract_startup_without_godi_with_resources(
        store,
        limits.startup,
        super::resources::Profile::SourceExpandedV2,
    )
    .map_err(Error::Startup)?;
    let world = available_70374::extract_all_with_source_expanded_v2_material_filtered_wmo_70450(
        startup,
        sink,
        limits.world,
    )
    .map_err(Error::Available)?;
    Ok(Completed {
        world,
        coverage: excluded_coverage(),
        gameobject_facts: None,
    })
}

#[cfg(target_os = "linux")]
pub fn extract_all_with_source_expanded_v2_material_filtered_wmo_70450(
    store: &mut LocalStore,
    sink: &mut impl run::Sink,
    limits: Limits,
) -> Result<Completed, Error> {
    extract_all_with_source_expanded_v2_material_filtered_wmo(store, sink, limits)
}

#[cfg(target_os = "linux")]
pub fn extract_all_with_model_output_expanded_material_filtered_wmo(
    store: &mut LocalStore,
    sink: &mut impl run::Sink,
    limits: Limits,
) -> Result<Completed, Error> {
    target::Target::Build70450
        .check(store)
        .map_err(Error::Target)?;
    let startup = gameobjects::extract_startup_without_godi_with_resources(
        store,
        limits.startup,
        super::resources::Profile::ModelOutputExpandedV1,
    )
    .map_err(Error::Startup)?;
    let world =
        available_70374::extract_all_with_model_output_expanded_material_filtered_wmo_70450(
            startup,
            sink,
            limits.world,
        )
        .map_err(Error::Available)?;
    Ok(Completed {
        world,
        coverage: excluded_coverage(),
        gameobject_facts: None,
    })
}

#[cfg(target_os = "linux")]
pub fn extract_all_with_model_output_expanded_material_filtered_wmo_70450(
    store: &mut LocalStore,
    sink: &mut impl run::Sink,
    limits: Limits,
) -> Result<Completed, Error> {
    extract_all_with_model_output_expanded_material_filtered_wmo(store, sink, limits)
}

#[cfg(target_os = "linux")]
pub fn extract_all_with_placement_expanded_material_filtered_wmo(
    store: &mut LocalStore,
    sink: &mut impl run::Sink,
    limits: Limits,
) -> Result<Completed, Error> {
    target::Target::Build70450
        .check(store)
        .map_err(Error::Target)?;
    let startup = gameobjects::extract_startup_without_godi_with_resources(
        store,
        limits.startup,
        super::resources::Profile::PlacementExpandedV1,
    )
    .map_err(Error::Startup)?;
    let world = available_70374::extract_all_with_placement_expanded_material_filtered_wmo_70450(
        startup,
        sink,
        limits.world,
    )
    .map_err(Error::Available)?;
    Ok(Completed {
        world,
        coverage: excluded_coverage(),
        gameobject_facts: None,
    })
}

#[cfg(target_os = "linux")]
pub fn extract_all_with_placement_expanded_material_filtered_wmo_70450(
    store: &mut LocalStore,
    sink: &mut impl run::Sink,
    limits: Limits,
) -> Result<Completed, Error> {
    extract_all_with_placement_expanded_material_filtered_wmo(store, sink, limits)
}

#[cfg(target_os = "linux")]
pub fn extract_all_with_assembly_expanded_material_filtered_wmo(
    store: &mut LocalStore,
    sink: &mut impl run::Sink,
    limits: Limits,
) -> Result<Completed, Error> {
    target::Target::Build70450
        .check(store)
        .map_err(Error::Target)?;
    let startup = gameobjects::extract_startup_without_godi_with_resources(
        store,
        limits.startup,
        super::resources::Profile::AssemblyExpandedV1,
    )
    .map_err(Error::Startup)?;
    let world = available_70374::extract_all_with_assembly_expanded_material_filtered_wmo_70450(
        startup,
        sink,
        limits.world,
    )
    .map_err(Error::Available)?;
    Ok(Completed {
        world,
        coverage: excluded_coverage(),
        gameobject_facts: None,
    })
}

#[cfg(target_os = "linux")]
pub fn extract_all_with_assembly_expanded_material_filtered_wmo_70450(
    store: &mut LocalStore,
    sink: &mut impl run::Sink,
    limits: Limits,
) -> Result<Completed, Error> {
    extract_all_with_assembly_expanded_material_filtered_wmo(store, sink, limits)
}

#[cfg(target_os = "linux")]
pub fn extract_all_with_gameobject_expanded_material_filtered_wmo(
    store: &mut LocalStore,
    sink: &mut impl run::Sink,
    limits: Limits,
) -> Result<Completed, Error> {
    extract_gameobject_material_filtered_wmo(store, sink, limits, GameObjectStartupPolicy::Strict)
}

#[cfg(target_os = "linux")]
pub fn extract_all_with_gameobject_missing_root_material_filtered_wmo(
    store: &mut LocalStore,
    sink: &mut impl run::Sink,
    limits: Limits,
) -> Result<Completed, Error> {
    extract_gameobject_material_filtered_wmo(
        store,
        sink,
        limits,
        GameObjectStartupPolicy::MissingRoot,
    )
}

#[cfg(target_os = "linux")]
pub fn extract_all_with_gameobject_root_or_encrypted_unavailable_material_filtered_wmo(
    store: &mut LocalStore,
    sink: &mut impl run::Sink,
    limits: Limits,
) -> Result<Completed, Error> {
    extract_gameobject_material_filtered_wmo(
        store,
        sink,
        limits,
        GameObjectStartupPolicy::RootOrEncryptedUnavailable,
    )
}

#[cfg(target_os = "linux")]
#[derive(Clone, Copy, PartialEq, Eq)]
enum GameObjectStartupPolicy {
    Strict,
    MissingRoot,
    RootOrEncryptedUnavailable,
}

#[cfg(target_os = "linux")]
fn extract_gameobject_material_filtered_wmo(
    store: &mut LocalStore,
    sink: &mut impl run::Sink,
    limits: Limits,
    gameobject_policy: GameObjectStartupPolicy,
) -> Result<Completed, Error> {
    target::Target::Build70450
        .check(store)
        .map_err(Error::Target)?;
    let allow_missing_root = gameobject_policy != GameObjectStartupPolicy::Strict;
    let allow_encrypted_unavailable =
        gameobject_policy == GameObjectStartupPolicy::RootOrEncryptedUnavailable;
    let startup = match gameobject_policy {
        GameObjectStartupPolicy::Strict => {
            available_70374::extract_startup_fdid_70450(store, limits.startup)
        }
        GameObjectStartupPolicy::MissingRoot => {
            available_70374::extract_startup_fdid_70450_with_gameobject_root_absence(
                store,
                limits.startup,
            )
        }
        GameObjectStartupPolicy::RootOrEncryptedUnavailable => {
            available_70374::extract_startup_fdid_70450_with_root_or_encrypted_unavailable(
                store,
                limits.startup,
            )
        }
    }
    .map_err(Error::Available)?;
    let world = if allow_encrypted_unavailable {
        available_70374::extract_all_with_gameobject_root_or_encrypted_unavailable_material_filtered_wmo_70450(
            startup,
            sink,
            limits.world,
        )
    } else if allow_missing_root {
        available_70374::extract_all_with_gameobject_missing_root_material_filtered_wmo_70450(
            startup,
            sink,
            limits.world,
        )
    } else {
        available_70374::extract_all_with_gameobject_expanded_material_filtered_wmo_70450(
            startup,
            sink,
            limits.world,
        )
    }
    .map_err(Error::Available)?;
    let data = world.startup().data();
    let table = data.table();
    let stats = data.stats();
    let lists = data.lists();
    let mut sections = Vec::new();
    sections
        .try_reserve_exact(data.section_facts().size_hint().0)
        .map_err(|_| Error::Available(available_70374::Error::Limit))?;
    sections.extend(data.section_facts());
    let mut outcomes = Vec::new();
    outcomes
        .try_reserve_exact(data.outcomes().len())
        .map_err(|_| Error::Available(available_70374::Error::Limit))?;
    outcomes.extend(data.outcome_facts());
    let facts = GameObjectFacts {
        declared_bytes: table.declared_bytes,
        available_bytes: table.available_bytes,
        full_ckey_verified: table.full_ckey_verified,
        encoded_key_authenticated: table.encoded_key_authenticated,
        declared_direct_rows: table.declared_direct_rows as u64,
        declared_copy_records: table.declared_copy_records as u64,
        direct_rows: table.direct_rows as u64,
        copy_records: table.copy_records as u64,
        materialized_rows: table.materialized_rows as u64,
        candidate_requests: stats.candidate_requests as u64,
        direct_outcomes: outcomes.len() as u64,
        zero_fdids: stats.zero_fdids as u64,
        textures: stats.textures as u64,
        m2_no_collision: stats.m2_no_collision as u64,
        wmo_doodad_only: stats.wmo_doodad_only as u64,
        missing_root_rows: data.missing_root_rows() as u64,
        missing_root_fdids: world.gameobject_missing_root().to_vec(),
        encrypted_unavailable_rows: data.encrypted_unavailable_rows() as u64,
        encrypted_unavailable_fdids: data.encrypted_unavailable().to_vec(),
        collision_rows: stats.collision_rows as u64,
        list_rows: lists.stats().rows as u64,
        list_m2_rows: lists.stats().m2_rows as u64,
        list_wmo_rows: lists.stats().wmo_rows as u64,
        list_vertex_visits: lists.stats().vertex_visits,
        list_raw_visit_bytes: lists.stats().raw_visit_bytes,
        temp_bytes: lists.temp_bytes().len() as u64,
        dtree_bytes: lists.dtree_bytes().len() as u64,
        temp_sha256: crate::integrity::digest(lists.temp_bytes()),
        dtree_sha256: crate::integrity::digest(lists.dtree_bytes()),
        work: table.work(),
        sections,
        outcomes,
    };
    let coverage = CoverageFacts {
        gameobject_scope: match gameobject_policy {
            GameObjectStartupPolicy::Strict => GameObjectScope::AvailableFdidDirectV1,
            GameObjectStartupPolicy::MissingRoot => {
                GameObjectScope::AvailableFdidDirectMissingRootV1
            }
            GameObjectStartupPolicy::RootOrEncryptedUnavailable => {
                GameObjectScope::AvailableFdidDirectRootAbsentOrEncryptedUnavailableV1
            }
        },
        gameobject_file_data_id: table.file_data_id(),
        gameobject_block_reason: GameObjectBlockReason::None,
        gameobject_request_made: true,
        gameobject_startup_reads: world.startup_reads() as u64,
        gameobject_sections: facts.sections.len() as u64,
        gameobject_direct_rows: facts.direct_rows,
        gameobject_copy_records: facts.copy_records,
        gameobject_outcomes: facts.direct_outcomes,
        model_scope: match gameobject_policy {
            GameObjectStartupPolicy::Strict => "placement-and-gameobject-v1",
            GameObjectStartupPolicy::MissingRoot => "placement-and-gameobject-with-missing-root-v1",
            GameObjectStartupPolicy::RootOrEncryptedUnavailable => {
                "placement-and-gameobject-with-root-absent-or-encrypted-unavailable-v1"
            }
        },
        liquid_object_scope: LiquidObjectScope::OutOfScope,
        liquid_object_request_made: false,
    };
    Ok(Completed {
        world,
        coverage,
        gameobject_facts: Some(facts),
    })
}

#[cfg(target_os = "linux")]
pub fn extract_all_with_gameobject_expanded_material_filtered_wmo_70450(
    store: &mut LocalStore,
    sink: &mut impl run::Sink,
    limits: Limits,
) -> Result<Completed, Error> {
    extract_all_with_gameobject_expanded_material_filtered_wmo(store, sink, limits)
}

#[cfg(target_os = "linux")]
pub fn extract_all_with_gameobject_missing_root_material_filtered_wmo_70450(
    store: &mut LocalStore,
    sink: &mut impl run::Sink,
    limits: Limits,
) -> Result<Completed, Error> {
    extract_all_with_gameobject_missing_root_material_filtered_wmo(store, sink, limits)
}

#[cfg(target_os = "linux")]
pub fn extract_all_with_gameobject_root_or_encrypted_unavailable_material_filtered_wmo_70450(
    store: &mut LocalStore,
    sink: &mut impl run::Sink,
    limits: Limits,
) -> Result<Completed, Error> {
    extract_all_with_gameobject_root_or_encrypted_unavailable_material_filtered_wmo(
        store, sink, limits,
    )
}

#[cfg(target_os = "linux")]
pub fn extract_all_with_model_bytes_expanded_material_filtered_wmo(
    store: &mut LocalStore,
    sink: &mut impl run::Sink,
    limits: Limits,
) -> Result<Completed, Error> {
    target::Target::Build70450
        .check(store)
        .map_err(Error::Target)?;
    let startup = gameobjects::extract_startup_without_godi_with_resources(
        store,
        limits.startup,
        super::resources::Profile::ModelBytesExpandedV1,
    )
    .map_err(Error::Startup)?;
    let world = available_70374::extract_all_with_model_bytes_expanded_material_filtered_wmo_70450(
        startup,
        sink,
        limits.world,
    )
    .map_err(Error::Available)?;
    Ok(Completed {
        world,
        coverage: excluded_coverage(),
        gameobject_facts: None,
    })
}

#[cfg(target_os = "linux")]
pub fn extract_all_with_model_bytes_expanded_material_filtered_wmo_70450(
    store: &mut LocalStore,
    sink: &mut impl run::Sink,
    limits: Limits,
) -> Result<Completed, Error> {
    extract_all_with_model_bytes_expanded_material_filtered_wmo(store, sink, limits)
}

#[cfg(test)]
mod tests;
