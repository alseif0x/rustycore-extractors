// SPDX-License-Identifier: GPL-3.0-or-later
// Modified by the RustyCore Extractors project, 2026-10-19.
// Modified 2026-10-19: strict empty-startup sentinel and target/profile binding;
// authored 2026-10-15: separate Linux70374 available DB2 startup/world owner.
//! Available GODI/Map coverage stays distinct from complete source facts. Existing
//! APIs reject missing dependencies. Separate opt-in profiles declare only WDT
//! or WMO-doodad M2 FDIDs whose raw ROOT has no rows; v34 can also declare a
//! strictly proven intrinsically empty WMO, v35 can declare valid source
//! geometry skipped before selection, and v36 can declare geometry removed by
//! the production material filter. Other sources remain strict;
//! there is no package publication or runtime claim. All phases move one
//! collector/cache/ledger through lookup transitions. Sink callbacks remain
//! provisional until whole completion.
#![cfg(target_os = "linux")]

use super::{Extraction, Extractor, Facts, Model, ModelHandle, gameobjects, plan, run, world};
use crate::local_store::{LocalStore, availability as acquired};
use forever_db2::{RecordError, availability as db2};
use forever_vmap::{gameobject_models, wmo::liquid::LiquidLookup};

/// Separate available-section startup over the existing private payload spool.
pub mod spooled;

#[cfg(test)]
mod tests;

pub const PROFILE: &str = "linux70374-disabled-key-available-collision-world-v1";
pub const PROFILE_70450_SELECTED_GEOMETRY: &str = "linux70450-selected-map-geometry-collision-v1";
pub const PROFILE_70450_AVAILABLE_GEOMETRY: &str = "linux70450-available-map-geometry-collision-v1";
pub const PROFILE_70450_AVAILABLE_ROOT_ABSENT_GEOMETRY: &str =
    "linux70450-available-map-root-omissions-collision-v1";
pub const PROFILE_70450_AVAILABLE_MODEL_ROOT_ABSENT_GEOMETRY: &str =
    "linux70450-available-map-model-root-omissions-collision-v1";
pub const PROFILE_70450_AVAILABLE_MODEL_ROOT_ABSENT_REQUEST_CAPACITY_GEOMETRY: &str =
    "linux70450-available-map-model-root-omissions-request-capacity-collision-v1";
pub const PROFILE_70450_AVAILABLE_VACATED_WMO_GEOMETRY: &str =
    "linux70450-available-vacated-wmo-collision-v1";
pub const PROFILE_70450_AVAILABLE_EMPTY_WMO_GEOMETRY: &str =
    "linux70450-available-empty-wmo-collision-v1";
pub const PROFILE_70450_AVAILABLE_UNSELECTED_WMO_GEOMETRY: &str =
    "linux70450-available-unselected-wmo-collision-v1";
pub const PROFILE_70450_AVAILABLE_MATERIAL_FILTERED_WMO_GEOMETRY: &str =
    "linux70450-available-material-filtered-wmo-collision-v1";
pub const PROFILE_70450_AVAILABLE_DEPENDENCY_HEAVY_MATERIAL_FILTERED_WMO_GEOMETRY: &str =
    "linux70450-available-dependency-heavy-material-filtered-wmo-collision-v1";
pub const PROFILE_70450_AVAILABLE_SOURCE_HEAVY_MATERIAL_FILTERED_WMO_GEOMETRY: &str =
    "linux70450-available-source-heavy-material-filtered-wmo-collision-v1";
pub const PROFILE_70450_AVAILABLE_READ_HEAVY_MATERIAL_FILTERED_WMO_GEOMETRY: &str =
    "linux70450-available-read-heavy-material-filtered-wmo-collision-v1";
pub const PROFILE_70450_AVAILABLE_MODEL_HEAVY_MATERIAL_FILTERED_WMO_GEOMETRY: &str =
    "linux70450-available-model-heavy-material-filtered-wmo-collision-v1";
pub const PROFILE_70450_AVAILABLE_SOURCE_EXPANDED_MATERIAL_FILTERED_WMO_GEOMETRY: &str =
    "linux70450-available-source-expanded-material-filtered-wmo-collision-v1";
pub const PROFILE_70450_AVAILABLE_MODEL_OUTPUT_EXPANDED_MATERIAL_FILTERED_WMO_GEOMETRY: &str =
    "linux70450-available-model-output-expanded-material-filtered-wmo-collision-v1";
pub const PROFILE_70450_AVAILABLE_MODEL_BYTES_EXPANDED_MATERIAL_FILTERED_WMO_GEOMETRY: &str =
    "linux70450-available-model-bytes-expanded-material-filtered-wmo-collision-v1";
pub const PROFILE_70450_AVAILABLE_SOURCE_EXPANDED_V2_MATERIAL_FILTERED_WMO_GEOMETRY: &str =
    "linux70450-available-source-expanded-v2-material-filtered-wmo-collision-v1";
pub const PROFILE_70450_AVAILABLE_PLACEMENT_EXPANDED_MATERIAL_FILTERED_WMO_GEOMETRY: &str =
    "linux70450-available-placement-expanded-material-filtered-wmo-collision-v1";
pub const PROFILE_70450_AVAILABLE_ASSEMBLY_EXPANDED_MATERIAL_FILTERED_WMO_GEOMETRY: &str =
    "linux70450-available-assembly-expanded-material-filtered-wmo-collision-v1";
pub const PROFILE_70450_AVAILABLE_GAMEOBJECT_EXPANDED_MATERIAL_FILTERED_WMO_GEOMETRY: &str =
    "linux70450-available-gameobject-expanded-material-filtered-wmo-collision-v1";
pub const PROFILE_70450_AVAILABLE_GAMEOBJECT_MISSING_ROOT_MATERIAL_FILTERED_WMO_GEOMETRY: &str =
    "linux70450-available-gameobject-missing-root-material-filtered-wmo-collision-v1";
pub const PROFILE_70450_AVAILABLE_GAMEOBJECT_ROOT_ENCRYPTED_UNAVAILABLE_MATERIAL_FILTERED_WMO_GEOMETRY: &str =
    "linux70450-available-gameobject-root-encrypted-unavailable-material-filtered-wmo-collision-v1";
static EMPTY: gameobjects::EmptyLookup = gameobjects::EmptyLookup;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Error {
    Limit,
    State,
    Db2(RecordError),
    Collision(super::Error),
    Startup(gameobjects::Error),
    World(world::Error),
    Plan(plan::Error),
    Run(run::Error),
}
impl Error {
    pub fn code(self) -> &'static str {
        match self {
            Self::Limit => "available_collision_policy_limit",
            Self::State => "available_collision_state",
            Self::Db2(e) => e.code(),
            Self::Collision(e) => e.code(),
            Self::Startup(e) => e.code(),
            Self::World(e) => e.code(),
            Self::Plan(e) => e.code(),
            Self::Run(e) => e.code(),
        }
    }
}
impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.code())
    }
}
impl std::error::Error for Error {}

/// Producer-reported available section facts, never complete SourceFacts.
/// No raw buffer, content key or Debug representation is provided.
pub struct TableFacts {
    file_data_id: u32,
    pub declared_bytes: u64,
    pub available_bytes: u64,
    pub full_ckey_verified: bool,
    pub encoded_key_authenticated: bool,
    pub work: acquired::Work,
    pub declared_direct_rows: usize,
    pub declared_copy_records: usize,
    pub direct_rows: usize,
    pub copy_records: usize,
    pub materialized_rows: usize,
    pub retained_bytes: usize,
    pub decode_allowance_bytes: usize,
    sections: [Option<db2::SectionCoverage>; 16],
}
impl TableFacts {
    pub const fn file_data_id(&self) -> u32 {
        self.file_data_id
    }
    pub fn sections(&self) -> impl Iterator<Item = &db2::SectionCoverage> {
        self.sections.iter().flatten()
    }
    pub fn work(&self) -> acquired::Work {
        self.work
    }
    pub fn all_sections_admitted(&self) -> bool {
        self.sections()
            .all(|s| s.state == db2::SectionState::Admitted)
    }
}
pub struct Data {
    table: TableFacts,
    engine: gameobjects::EngineData,
    missing_root: Vec<u32>,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SectionFact {
    pub ordinal: u32,
    pub declared_records: u32,
    pub declared_copies: u32,
    pub admitted: bool,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GameObjectOutcome {
    ZeroFileDataId,
    Texture,
    Collision,
    M2NoCollision,
    WmoDoodadOnly,
    MissingRoot,
    EncryptedUnavailable,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct GameObjectOutcomeFact {
    pub ordinal: u64,
    pub display_id: u32,
    pub file_data_id: u32,
    pub outcome: GameObjectOutcome,
}
impl Data {
    pub const fn table(&self) -> &TableFacts {
        &self.table
    }
    pub fn outcomes(&self) -> &[gameobjects::RowOutcome] {
        &self.engine.outcomes
    }
    pub const fn lists(&self) -> &gameobject_models::Lists {
        &self.engine.lists
    }
    pub const fn stats(&self) -> gameobjects::Stats {
        self.engine.stats
    }
    pub const fn startup_facts(&self) -> Facts {
        self.engine.startup_facts
    }
    pub fn missing_root(&self) -> &[u32] {
        &self.missing_root
    }
    pub fn encrypted_unavailable(&self) -> &[u32] {
        &self.engine.encrypted_unavailable
    }
    pub fn encrypted_unavailable_rows(&self) -> usize {
        self.engine.stats.encrypted_unavailable_rows
    }
    pub fn missing_root_rows(&self) -> usize {
        self.engine
            .outcomes
            .iter()
            .filter(|outcome| matches!(outcome.outcome(), gameobjects::Outcome::MissingRoot))
            .count()
    }
    pub fn section_facts(&self) -> impl Iterator<Item = SectionFact> + '_ {
        self.table.sections().map(|section| SectionFact {
            ordinal: section.ordinal,
            declared_records: section.declared_records,
            declared_copies: section.declared_copies,
            admitted: section.state == db2::SectionState::Admitted,
        })
    }
    pub fn outcome_facts(&self) -> impl Iterator<Item = GameObjectOutcomeFact> + '_ {
        self.engine
            .outcomes
            .iter()
            .map(|outcome| GameObjectOutcomeFact {
                ordinal: outcome.ordinal() as u64,
                display_id: outcome.display_id(),
                file_data_id: outcome.file_data_id(),
                outcome: match outcome.outcome() {
                    gameobjects::Outcome::ZeroFileDataId => GameObjectOutcome::ZeroFileDataId,
                    gameobjects::Outcome::Texture => GameObjectOutcome::Texture,
                    gameobjects::Outcome::Collision(_) => GameObjectOutcome::Collision,
                    gameobjects::Outcome::M2NoCollision(_) => GameObjectOutcome::M2NoCollision,
                    gameobjects::Outcome::WmoDoodadOnly(_) => GameObjectOutcome::WmoDoodadOnly,
                    gameobjects::Outcome::MissingRoot => GameObjectOutcome::MissingRoot,
                    gameobjects::Outcome::EncryptedUnavailable => {
                        GameObjectOutcome::EncryptedUnavailable
                    }
                },
            })
    }
    pub fn collision_model_handles(
        &self,
    ) -> impl Iterator<Item = (GameObjectOutcomeFact, ModelHandle)> + '_ {
        self.engine.outcomes.iter().filter_map(|outcome| {
            let handle = match outcome.outcome() {
                gameobjects::Outcome::Collision(handle) => handle,
                _ => return None,
            };
            Some((
                GameObjectOutcomeFact {
                    ordinal: outcome.ordinal() as u64,
                    display_id: outcome.display_id(),
                    file_data_id: outcome.file_data_id(),
                    outcome: GameObjectOutcome::Collision,
                },
                handle,
            ))
        })
    }
}
pub type Startup<'store> = Context<'store, 'static, gameobjects::EmptyLookup>;
pub struct Context<'store, 'lookup, L: LiquidLookup + ?Sized> {
    data: Data,
    collector: Extractor<'store, 'lookup, L>,
}
impl<'store, 'lookup, L: LiquidLookup + ?Sized> Context<'store, 'lookup, L> {
    pub const fn data(&self) -> &Data {
        &self.data
    }
    pub fn model(&self, handle: ModelHandle) -> Option<&Model> {
        self.collector.model(handle)
    }
    pub(crate) const fn resources(&self) -> crate::collision::resources::Profile {
        self.collector.resources()
    }
    pub fn collector_mut(&mut self) -> &mut Extractor<'store, 'lookup, L> {
        &mut self.collector
    }
    pub fn with_lookup<'next, N: LiquidLookup + ?Sized>(
        self,
        lookup: &'next N,
    ) -> Result<Context<'store, 'next, N>, Error> {
        Ok(Context {
            data: self.data,
            collector: self
                .collector
                .with_lookup(lookup)
                .map_err(Error::Collision)?,
        })
    }
    pub fn finish(mut self) -> Result<CompletedStartup, Error> {
        self.collector
            .release_external(self.data.engine.stats.resident_bytes)
            .map_err(Error::Collision)?;
        Ok(CompletedStartup {
            data: self.data,
            extraction: self.collector.finish().map_err(Error::Collision)?,
        })
    }
}
pub struct CompletedStartup {
    data: Data,
    extraction: Extraction,
}
impl CompletedStartup {
    pub const fn data(&self) -> &Data {
        &self.data
    }
    pub const fn extraction(&self) -> &Extraction {
        &self.extraction
    }
}

/// Build 70374 physical admitted direct GODI rows only. Copies are
/// checked/countable but never trigger model acquisition.
pub fn extract_startup_fdid(
    store: &mut LocalStore,
    limits: gameobjects::Limits,
) -> Result<Startup<'_>, Error> {
    extract_startup_fdid_with_target(
        store,
        limits,
        super::target::Target::Build70374,
        crate::collision::resources::Profile::BoundedV1,
        false,
        false,
    )
}

/// Build 70450 physical admitted direct GODI rows for the v47 producer path.
#[cfg(target_os = "linux")]
pub fn extract_startup_fdid_70450(
    store: &mut LocalStore,
    limits: gameobjects::Limits,
) -> Result<Startup<'_>, Error> {
    super::target::Target::Build70450
        .check(store)
        .map_err(Error::Collision)?;
    extract_startup_fdid_with_target(
        store,
        limits,
        super::target::Target::Build70450,
        crate::collision::resources::Profile::GameObjectExpandedV1,
        false,
        false,
    )
}

#[cfg(target_os = "linux")]
pub fn extract_startup_fdid_70450_with_gameobject_root_absence(
    store: &mut LocalStore,
    limits: gameobjects::Limits,
) -> Result<Startup<'_>, Error> {
    super::target::Target::Build70450
        .check(store)
        .map_err(Error::Collision)?;
    extract_startup_fdid_with_target(
        store,
        limits,
        super::target::Target::Build70450,
        crate::collision::resources::Profile::GameObjectMissingRootExpandedV1,
        true,
        false,
    )
}

#[cfg(target_os = "linux")]
pub fn extract_startup_fdid_70450_with_root_or_encrypted_unavailable(
    store: &mut LocalStore,
    limits: gameobjects::Limits,
) -> Result<Startup<'_>, Error> {
    super::target::Target::Build70450
        .check(store)
        .map_err(Error::Collision)?;
    extract_startup_fdid_with_target(
        store,
        limits,
        super::target::Target::Build70450,
        crate::collision::resources::Profile::GameObjectRootAndEncryptedUnavailableExpandedV1,
        true,
        true,
    )
}

#[cfg(target_os = "linux")]
pub fn extract_startup_fdid_70450_with_root_absence(
    store: &mut LocalStore,
    limits: gameobjects::Limits,
) -> Result<Startup<'_>, Error> {
    extract_startup_fdid_70450_with_gameobject_root_absence(store, limits)
}

fn extract_startup_fdid_with_target(
    store: &mut LocalStore,
    limits: gameobjects::Limits,
    target: super::target::Target,
    resource: crate::collision::resources::Profile,
    prove_godi_root_absent: bool,
    encrypted_unavailable_profile: bool,
) -> Result<Startup<'_>, Error> {
    limits
        .validate_resources(resource)
        .map_err(Error::Startup)?;
    let mut collector = Extractor::new_with_resources(store, &EMPTY, limits.collision, resource)
        .map_err(Error::Collision)?;
    let file = collector
        .read_available_source_target(
            gameobjects::TABLE_FILE_DATA_ID,
            limits.max_db2_bytes,
            target,
        )
        .map_err(Error::Collision)?;
    let source_resident = file
        .resident_bytes()
        .map_err(|e| Error::Collision(super::Error::from_available(e)))?;
    header_policy(&file, limits.max_direct_rows, limits.max_copy_records)?;
    let allowance = allowance(
        file.declared_bytes(),
        false,
        limits.max_decode_allowance_bytes,
    )?;
    collector
        .reserve_external(allowance)
        .map_err(Error::Collision)?;
    let snapshot = match target {
        super::target::Target::Build70374 => db2::read_gameobject_display_info_70374(&file),
        super::target::Target::Build70425 => db2::read_gameobject_display_info_70425(&file),
        super::target::Target::Build70450 => db2::read_gameobject_display_info_70450(&file),
    }
    .map_err(Error::Db2)?;
    let mut table = table_facts(
        &file,
        snapshot.sections(),
        snapshot.direct_records().len(),
        snapshot.copies().len(),
        allowance,
    )?;
    if table.declared_direct_rows > limits.max_direct_rows
        || table.declared_copy_records > limits.max_copy_records
    {
        return Err(Error::Limit);
    }
    let gameobjects::DirectRows {
        outcomes,
        stats,
        mut missing_root,
        encrypted_unavailable,
    } = gameobjects::collect_direct(
        &mut collector,
        snapshot.direct_records(),
        snapshot.copies().len(),
        allowance,
        limits,
        prove_godi_root_absent,
        encrypted_unavailable_profile,
    )
    .map_err(Error::Startup)?;
    missing_root.sort_unstable();
    missing_root.dedup();
    let missing_root_capacity = capacity(&missing_root)?;
    drop(snapshot);
    drop(file);
    collector
        .release_external(allowance)
        .map_err(Error::Collision)?;
    collector
        .release_source(source_resident)
        .map_err(Error::Collision)?;
    let mut engine = gameobjects::finish_lists(
        &mut collector,
        outcomes,
        stats,
        limits,
        encrypted_unavailable,
    )
    .map_err(Error::Startup)?;
    engine.stats.resident_bytes = add(engine.stats.resident_bytes, missing_root_capacity)?;
    table.materialized_rows = table.direct_rows; // No copy replay for GODI.
    Ok(Context {
        data: Data {
            table,
            engine,
            missing_root,
        },
        collector,
    })
}

/// Topology of the admitted Map rows only, with an independently retained
/// available Map fact. This is not complete-source world::Completed.
pub struct Topology {
    inner: world::Topology,
}
impl Topology {
    /// Internal borrowed validated graph for resident assembly. This does not
    /// convert the available source/world owner into a complete-source owner.
    pub(in crate::collision) const fn assembly_topology(&self) -> &world::Topology {
        &self.inner
    }
    pub fn all(&self) -> &[world::TopologyEntry] {
        self.inner.all()
    }
    pub fn get(&self, id: u32) -> Option<&world::TopologyEntry> {
        self.inner.get(id)
    }
    pub const fn selection(&self) -> plan::MapSelection {
        self.inner.selection()
    }
    pub fn contains_selected(&self, id: u32) -> bool {
        self.inner.contains_selected(id)
    }
    pub fn selected_active(&self) -> impl Iterator<Item = &world::TopologyEntry> {
        self.inner.selected_active()
    }
    pub const fn direct_count(&self) -> usize {
        self.inner.direct_count()
    }
    pub const fn resident_bytes(&self) -> usize {
        self.inner.resident_bytes()
    }
}
pub struct Completed {
    profile: &'static str,
    startup: CompletedStartup,
    report: run::SharedReport,
    root_absent: Vec<run::RootAbsent>,
    missing_m2: Vec<u32>,
    missing_wmo: Vec<u32>,
    empty_wmo: Vec<u32>,
    unselected_wmo: Vec<u32>,
    material_filtered_wmo: Vec<u32>,
    map: TableFacts,
    liquids: [world::TableFacts; 2],
    lookups: world::LookupStats,
    maps: plan::Stats,
    topology: Topology,
    resident_bytes: usize,
    startup_reads: usize,
}
impl Completed {
    pub const fn profile(&self) -> &'static str {
        self.profile
    }
    pub const fn startup(&self) -> &CompletedStartup {
        &self.startup
    }
    pub const fn report(&self) -> &run::SharedReport {
        &self.report
    }
    pub fn root_absent(&self) -> &[run::RootAbsent] {
        &self.root_absent
    }
    pub fn missing_m2(&self) -> &[u32] {
        &self.missing_m2
    }
    pub fn missing_wmo(&self) -> &[u32] {
        &self.missing_wmo
    }
    pub fn empty_wmo(&self) -> &[u32] {
        &self.empty_wmo
    }
    pub fn unselected_wmo(&self) -> &[u32] {
        &self.unselected_wmo
    }
    pub fn material_filtered_wmo(&self) -> &[u32] {
        &self.material_filtered_wmo
    }
    pub fn gameobject_missing_root(&self) -> &[u32] {
        self.startup.data().missing_root()
    }
    pub const fn map(&self) -> &TableFacts {
        &self.map
    }
    /// Complete CKey source facts in original LiquidMaterial/LiquidType order.
    pub const fn liquids(&self) -> &[world::TableFacts; 2] {
        &self.liquids
    }
    pub const fn lookup_stats(&self) -> world::LookupStats {
        self.lookups
    }
    pub const fn map_stats(&self) -> plan::Stats {
        self.maps
    }
    pub const fn topology(&self) -> &Topology {
        &self.topology
    }
    pub const fn resident_bytes(&self) -> usize {
        self.resident_bytes
    }
    pub const fn store_resident_bytes(&self) -> usize {
        self.startup.extraction().store_resident_bytes()
    }
    pub const fn startup_reads(&self) -> usize {
        self.startup_reads
    }
}

/// Consume the successful available startup; Map first, then complete liquid
/// sources, strict graph, and the unchanged strict shared WDT/ADT/model runner.
/// No partial Completed can escape a failure. Caller Sink storage is separate.
pub fn extract(
    startup: Startup<'_>,
    sink: &mut impl run::Sink,
    limits: world::Limits,
) -> Result<Completed, Error> {
    extract_with_selection(
        startup,
        sink,
        limits,
        super::target::Target::Build70374,
        plan::MapSelection::All,
    )
}

/// Available Build 70450 runner over every admitted active VMap row. Unknown
/// sections remain unknown and no full-client or full-world claim is made.
pub(in crate::collision) fn extract_all_70450(
    startup: gameobjects::Startup<'_>,
    sink: &mut impl run::Sink,
    limits: world::Limits,
) -> Result<Completed, Error> {
    extract_with_selection(
        empty_startup(startup)?,
        sink,
        limits,
        super::target::Target::Build70450,
        plan::MapSelection::All,
    )
}

pub(in crate::collision) fn extract_all_with_root_absence_70450(
    startup: gameobjects::Startup<'_>,
    sink: &mut impl run::Sink,
    limits: world::Limits,
) -> Result<Completed, Error> {
    extract_with_policy(
        empty_startup(startup)?,
        sink,
        limits,
        super::target::Target::Build70450,
        plan::MapSelection::All,
        run::MissingWdtPolicy::DeclareRootAbsent,
    )
}

pub(in crate::collision) fn extract_all_with_model_root_absence_70450(
    startup: gameobjects::Startup<'_>,
    sink: &mut impl run::Sink,
    limits: world::Limits,
) -> Result<Completed, Error> {
    extract_with_omissions(
        empty_startup(startup)?,
        sink,
        limits,
        super::target::Target::Build70450,
        plan::MapSelection::All,
        OmissionOptions {
            missing_wdt: run::MissingWdtPolicy::DeclareRootAbsent,
            missing_model: run::MissingModelPolicy::DeclareRootAbsentWmoDoodadM2,
            missing_wmo: run::MissingWmoPolicy::Reject,
            resource: crate::collision::resources::Profile::BoundedV1,
        },
    )
}

pub(in crate::collision) fn extract_all_with_model_root_absence_request_capacity_70450(
    startup: gameobjects::Startup<'_>,
    sink: &mut impl run::Sink,
    limits: world::Limits,
) -> Result<Completed, Error> {
    extract_with_omissions(
        empty_startup(startup)?,
        sink,
        limits,
        super::target::Target::Build70450,
        plan::MapSelection::All,
        OmissionOptions {
            missing_wdt: run::MissingWdtPolicy::DeclareRootAbsent,
            missing_model: run::MissingModelPolicy::DeclareRootAbsentWmoDoodadM2,
            missing_wmo: run::MissingWmoPolicy::Reject,
            resource: crate::collision::resources::Profile::RequestHeavyV1,
        },
    )
}

pub(in crate::collision) fn extract_all_with_vacated_wmo_70450(
    startup: gameobjects::Startup<'_>,
    sink: &mut impl run::Sink,
    limits: world::Limits,
) -> Result<Completed, Error> {
    extract_with_omissions(
        empty_startup(startup)?,
        sink,
        limits,
        super::target::Target::Build70450,
        plan::MapSelection::All,
        OmissionOptions {
            missing_wdt: run::MissingWdtPolicy::DeclareRootAbsent,
            missing_model: run::MissingModelPolicy::DeclareRootAbsentWmoDoodadM2,
            missing_wmo: run::MissingWmoPolicy::DeclareVacatedAfterRootAbsentM2,
            resource: crate::collision::resources::Profile::RequestHeavyV1,
        },
    )
}

pub(in crate::collision) fn extract_all_with_empty_wmo_70450(
    startup: gameobjects::Startup<'_>,
    sink: &mut impl run::Sink,
    limits: world::Limits,
) -> Result<Completed, Error> {
    extract_with_omissions(
        empty_startup(startup)?,
        sink,
        limits,
        super::target::Target::Build70450,
        plan::MapSelection::All,
        OmissionOptions {
            missing_wdt: run::MissingWdtPolicy::DeclareRootAbsent,
            missing_model: run::MissingModelPolicy::DeclareRootAbsentWmoDoodadM2,
            missing_wmo: run::MissingWmoPolicy::DeclareVacatedAfterRootAbsentM2OrEmpty,
            resource: crate::collision::resources::Profile::RequestHeavyV1,
        },
    )
}

pub(in crate::collision) fn extract_all_with_unselected_wmo_70450(
    startup: gameobjects::Startup<'_>,
    sink: &mut impl run::Sink,
    limits: world::Limits,
) -> Result<Completed, Error> {
    extract_with_omissions(
        empty_startup(startup)?,
        sink,
        limits,
        super::target::Target::Build70450,
        plan::MapSelection::All,
        OmissionOptions {
            missing_wdt: run::MissingWdtPolicy::DeclareRootAbsent,
            missing_model: run::MissingModelPolicy::DeclareRootAbsentWmoDoodadM2,
            missing_wmo: run::MissingWmoPolicy::DeclareVacatedAfterRootAbsentM2OrEmptyOrUnselected,
            resource: crate::collision::resources::Profile::RequestHeavyV1,
        },
    )
}

#[cfg(target_os = "linux")]
pub(in crate::collision) fn extract_all_with_material_filtered_wmo_70450(
    startup: gameobjects::Startup<'_>,
    sink: &mut impl run::Sink,
    limits: world::Limits,
) -> Result<Completed, Error> {
    extract_with_omissions(
        empty_startup(startup)?,
        sink,
        limits,
        super::target::Target::Build70450,
        plan::MapSelection::All,
        OmissionOptions {
            missing_wdt: run::MissingWdtPolicy::DeclareRootAbsent,
            missing_model: run::MissingModelPolicy::DeclareRootAbsentWmoDoodadM2,
            missing_wmo: run::MissingWmoPolicy::DeclareVacatedAfterRootAbsentM2OrEmptyOrUnselectedOrMaterialFiltered,
            resource: crate::collision::resources::Profile::RequestHeavyV1,
        },
    )
}

#[cfg(target_os = "linux")]
pub(in crate::collision) fn extract_all_with_dependency_heavy_material_filtered_wmo_70450(
    startup: gameobjects::Startup<'_>,
    sink: &mut impl run::Sink,
    limits: world::Limits,
) -> Result<Completed, Error> {
    extract_with_omissions(
        empty_startup(startup)?,
        sink,
        limits,
        super::target::Target::Build70450,
        plan::MapSelection::All,
        OmissionOptions {
            missing_wdt: run::MissingWdtPolicy::DeclareRootAbsent,
            missing_model: run::MissingModelPolicy::DeclareRootAbsentWmoDoodadM2,
            missing_wmo: run::MissingWmoPolicy::DeclareVacatedAfterRootAbsentM2OrEmptyOrUnselectedOrMaterialFiltered,
            resource: crate::collision::resources::Profile::DependencyHeavyV1,
        },
    )
}

#[cfg(target_os = "linux")]
pub(in crate::collision) fn extract_all_with_source_heavy_material_filtered_wmo_70450(
    startup: gameobjects::Startup<'_>,
    sink: &mut impl run::Sink,
    limits: world::Limits,
) -> Result<Completed, Error> {
    extract_with_omissions(
        empty_startup(startup)?,
        sink,
        limits,
        super::target::Target::Build70450,
        plan::MapSelection::All,
        OmissionOptions {
            missing_wdt: run::MissingWdtPolicy::DeclareRootAbsent,
            missing_model: run::MissingModelPolicy::DeclareRootAbsentWmoDoodadM2,
            missing_wmo: run::MissingWmoPolicy::DeclareVacatedAfterRootAbsentM2OrEmptyOrUnselectedOrMaterialFiltered,
            resource: crate::collision::resources::Profile::SourceHeavyV1,
        },
    )
}

#[cfg(target_os = "linux")]
pub(in crate::collision) fn extract_all_with_read_heavy_material_filtered_wmo_70450(
    startup: gameobjects::Startup<'_>,
    sink: &mut impl run::Sink,
    limits: world::Limits,
) -> Result<Completed, Error> {
    extract_with_omissions(
        empty_startup(startup)?,
        sink,
        limits,
        super::target::Target::Build70450,
        plan::MapSelection::All,
        OmissionOptions {
            missing_wdt: run::MissingWdtPolicy::DeclareRootAbsent,
            missing_model: run::MissingModelPolicy::DeclareRootAbsentWmoDoodadM2,
            missing_wmo: run::MissingWmoPolicy::DeclareVacatedAfterRootAbsentM2OrEmptyOrUnselectedOrMaterialFiltered,
            resource: crate::collision::resources::Profile::ReadHeavyV1,
        },
    )
}

#[cfg(target_os = "linux")]
pub(in crate::collision) fn extract_all_with_model_heavy_material_filtered_wmo_70450(
    startup: gameobjects::Startup<'_>,
    sink: &mut impl run::Sink,
    limits: world::Limits,
) -> Result<Completed, Error> {
    extract_with_omissions(
        empty_startup(startup)?,
        sink,
        limits,
        super::target::Target::Build70450,
        plan::MapSelection::All,
        OmissionOptions {
            missing_wdt: run::MissingWdtPolicy::DeclareRootAbsent,
            missing_model: run::MissingModelPolicy::DeclareRootAbsentWmoDoodadM2,
            missing_wmo: run::MissingWmoPolicy::DeclareVacatedAfterRootAbsentM2OrEmptyOrUnselectedOrMaterialFiltered,
            resource: crate::collision::resources::Profile::ModelHeavyV1,
        },
    )
}

#[cfg(target_os = "linux")]
pub(in crate::collision) fn extract_all_with_source_expanded_material_filtered_wmo_70450(
    startup: gameobjects::Startup<'_>,
    sink: &mut impl run::Sink,
    limits: world::Limits,
) -> Result<Completed, Error> {
    extract_with_omissions(
        empty_startup(startup)?,
        sink,
        limits,
        super::target::Target::Build70450,
        plan::MapSelection::All,
        OmissionOptions {
            missing_wdt: run::MissingWdtPolicy::DeclareRootAbsent,
            missing_model: run::MissingModelPolicy::DeclareRootAbsentWmoDoodadM2,
            missing_wmo: run::MissingWmoPolicy::DeclareVacatedAfterRootAbsentM2OrEmptyOrUnselectedOrMaterialFiltered,
            resource: crate::collision::resources::Profile::SourceExpandedV1,
        },
    )
}

#[cfg(target_os = "linux")]
pub(in crate::collision) fn extract_all_with_source_expanded_v2_material_filtered_wmo_70450(
    startup: gameobjects::Startup<'_>,
    sink: &mut impl run::Sink,
    limits: world::Limits,
) -> Result<Completed, Error> {
    extract_with_omissions(
        empty_startup(startup)?,
        sink,
        limits,
        super::target::Target::Build70450,
        plan::MapSelection::All,
        OmissionOptions {
            missing_wdt: run::MissingWdtPolicy::DeclareRootAbsent,
            missing_model: run::MissingModelPolicy::DeclareRootAbsentWmoDoodadM2,
            missing_wmo: run::MissingWmoPolicy::DeclareVacatedAfterRootAbsentM2OrEmptyOrUnselectedOrMaterialFiltered,
            resource: crate::collision::resources::Profile::SourceExpandedV2,
        },
    )
}

#[cfg(target_os = "linux")]
pub(in crate::collision) fn extract_all_with_placement_expanded_material_filtered_wmo_70450(
    startup: gameobjects::Startup<'_>,
    sink: &mut impl run::Sink,
    limits: world::Limits,
) -> Result<Completed, Error> {
    extract_with_omissions(
        empty_startup(startup)?,
        sink,
        limits,
        super::target::Target::Build70450,
        plan::MapSelection::All,
        OmissionOptions {
            missing_wdt: run::MissingWdtPolicy::DeclareRootAbsent,
            missing_model: run::MissingModelPolicy::DeclareRootAbsentWmoDoodadM2,
            missing_wmo: run::MissingWmoPolicy::DeclareVacatedAfterRootAbsentM2OrEmptyOrUnselectedOrMaterialFiltered,
            resource: crate::collision::resources::Profile::PlacementExpandedV1,
        },
    )
}

#[cfg(target_os = "linux")]
pub(in crate::collision) fn extract_all_with_assembly_expanded_material_filtered_wmo_70450(
    startup: gameobjects::Startup<'_>,
    sink: &mut impl run::Sink,
    limits: world::Limits,
) -> Result<Completed, Error> {
    extract_all_with_assembly_expanded_startup_70450(
        empty_startup(startup)?,
        sink,
        limits,
        crate::collision::resources::Profile::AssemblyExpandedV1,
        crate::collision::resources::Profile::AssemblyExpandedV1,
    )
}

#[cfg(target_os = "linux")]
pub(in crate::collision) fn extract_all_with_gameobject_expanded_material_filtered_wmo_70450(
    startup: Startup<'_>,
    sink: &mut impl run::Sink,
    limits: world::Limits,
) -> Result<Completed, Error> {
    extract_all_with_assembly_expanded_startup_70450(
        startup,
        sink,
        limits,
        crate::collision::resources::Profile::GameObjectExpandedV1,
        crate::collision::resources::Profile::GameObjectExpandedV1,
    )
}

#[cfg(target_os = "linux")]
pub(in crate::collision) fn extract_all_with_gameobject_missing_root_material_filtered_wmo_70450(
    startup: Startup<'_>,
    sink: &mut impl run::Sink,
    limits: world::Limits,
) -> Result<Completed, Error> {
    extract_all_with_assembly_expanded_startup_70450(
        startup,
        sink,
        limits,
        crate::collision::resources::Profile::GameObjectMissingRootExpandedV1,
        crate::collision::resources::Profile::GameObjectMissingRootExpandedV1,
    )
}

#[cfg(target_os = "linux")]
pub(in crate::collision) fn extract_all_with_gameobject_root_or_encrypted_unavailable_material_filtered_wmo_70450(
    startup: Startup<'_>,
    sink: &mut impl run::Sink,
    limits: world::Limits,
) -> Result<Completed, Error> {
    extract_all_with_assembly_expanded_startup_70450(
        startup,
        sink,
        limits,
        crate::collision::resources::Profile::GameObjectRootAndEncryptedUnavailableExpandedV1,
        crate::collision::resources::Profile::GameObjectRootAndEncryptedUnavailableExpandedV1,
    )
}

#[cfg(target_os = "linux")]
fn extract_all_with_assembly_expanded_startup_70450(
    startup: Startup<'_>,
    sink: &mut impl run::Sink,
    limits: world::Limits,
    collector_resource: crate::collision::resources::Profile,
    completion_resource: crate::collision::resources::Profile,
) -> Result<Completed, Error> {
    if startup.resources() != collector_resource {
        return Err(Error::State);
    }
    extract_with_omissions(
        startup,
        sink,
        limits,
        super::target::Target::Build70450,
        plan::MapSelection::All,
        OmissionOptions {
            missing_wdt: run::MissingWdtPolicy::DeclareRootAbsent,
            missing_model: run::MissingModelPolicy::DeclareRootAbsentWmoDoodadM2,
            missing_wmo: run::MissingWmoPolicy::DeclareVacatedAfterRootAbsentM2OrEmptyOrUnselectedOrMaterialFiltered,
            resource: completion_resource,
        },
    )
}

#[cfg(target_os = "linux")]
pub(in crate::collision) fn extract_all_with_model_output_expanded_material_filtered_wmo_70450(
    startup: gameobjects::Startup<'_>,
    sink: &mut impl run::Sink,
    limits: world::Limits,
) -> Result<Completed, Error> {
    extract_with_omissions(
        empty_startup(startup)?,
        sink,
        limits,
        super::target::Target::Build70450,
        plan::MapSelection::All,
        OmissionOptions {
            missing_wdt: run::MissingWdtPolicy::DeclareRootAbsent,
            missing_model: run::MissingModelPolicy::DeclareRootAbsentWmoDoodadM2,
            missing_wmo: run::MissingWmoPolicy::DeclareVacatedAfterRootAbsentM2OrEmptyOrUnselectedOrMaterialFiltered,
            resource: crate::collision::resources::Profile::ModelOutputExpandedV1,
        },
    )
}

#[cfg(target_os = "linux")]
pub(in crate::collision) fn extract_all_with_model_bytes_expanded_material_filtered_wmo_70450(
    startup: gameobjects::Startup<'_>,
    sink: &mut impl run::Sink,
    limits: world::Limits,
) -> Result<Completed, Error> {
    extract_with_omissions(
        empty_startup(startup)?,
        sink,
        limits,
        super::target::Target::Build70450,
        plan::MapSelection::All,
        OmissionOptions {
            missing_wdt: run::MissingWdtPolicy::DeclareRootAbsent,
            missing_model: run::MissingModelPolicy::DeclareRootAbsentWmoDoodadM2,
            missing_wmo: run::MissingWmoPolicy::DeclareVacatedAfterRootAbsentM2OrEmptyOrUnselectedOrMaterialFiltered,
            resource: crate::collision::resources::Profile::ModelBytesExpandedV1,
        },
    )
}

/// Available Build 70450 domain runner used by the geometry-only profile.
/// The selected map and its active ancestors are planned from the admitted
/// available-section Map rows; Map, LiquidMaterial, and LiquidType remain strict
/// complete inputs.
pub(in crate::collision) fn extract_selected_70450(
    startup: gameobjects::Startup<'_>,
    map_id: u32,
    sink: &mut impl run::Sink,
    limits: world::Limits,
) -> Result<Completed, Error> {
    extract_with_selection(
        empty_startup(startup)?,
        sink,
        limits,
        super::target::Target::Build70450,
        plan::MapSelection::MapAndAncestors(map_id),
    )
}

fn empty_startup(startup: gameobjects::Startup<'_>) -> Result<Startup<'_>, Error> {
    let data = startup.data();
    if data.source().file_data_id() != 0
        || data.stats().direct_rows != 0
        || data.stats().copy_records != 0
        || data.stats().candidate_requests != 0
        || !data.outcomes().is_empty()
        || data.lists().stats().rows != 0
        || data.startup_facts().reads != 0
    {
        return Err(Error::State);
    }
    let (engine, missing_root, collector) = startup.into_engine_parts();
    Ok(Context {
        data: Data {
            missing_root,
            table: TableFacts {
                file_data_id: 0,
                declared_bytes: 0,
                available_bytes: 0,
                full_ckey_verified: false,
                encoded_key_authenticated: false,
                work: acquired::Work::default(),
                declared_direct_rows: 0,
                declared_copy_records: 0,
                direct_rows: 0,
                copy_records: 0,
                materialized_rows: 0,
                retained_bytes: 0,
                decode_allowance_bytes: 0,
                sections: [None; 16],
            },
            engine,
        },
        collector,
    })
}

#[cfg(test)]
fn completion_profile(
    target: super::target::Target,
    selection: plan::MapSelection,
) -> Result<&'static str, Error> {
    completion_profile_with_policy(target, selection, run::MissingWdtPolicy::Reject)
}

#[cfg(test)]
fn completion_profile_with_policy(
    target: super::target::Target,
    selection: plan::MapSelection,
    missing_wdt: run::MissingWdtPolicy,
) -> Result<&'static str, Error> {
    completion_profile_with_omissions(
        target,
        selection,
        missing_wdt,
        run::MissingModelPolicy::Reject,
        run::MissingWmoPolicy::Reject,
        crate::collision::resources::Profile::BoundedV1,
    )
}

fn completion_profile_with_omissions(
    target: super::target::Target,
    selection: plan::MapSelection,
    missing_wdt: run::MissingWdtPolicy,
    missing_model: run::MissingModelPolicy,
    missing_wmo: run::MissingWmoPolicy,
    resource: crate::collision::resources::Profile,
) -> Result<&'static str, Error> {
    match (
        target,
        selection,
        missing_wdt,
        missing_model,
        missing_wmo,
        resource,
    ) {
        (
            super::target::Target::Build70374,
            plan::MapSelection::All,
            run::MissingWdtPolicy::Reject,
            run::MissingModelPolicy::Reject,
            run::MissingWmoPolicy::Reject,
            crate::collision::resources::Profile::BoundedV1,
        ) => Ok(PROFILE),
        (
            super::target::Target::Build70450,
            plan::MapSelection::All,
            run::MissingWdtPolicy::Reject,
            run::MissingModelPolicy::Reject,
            run::MissingWmoPolicy::Reject,
            crate::collision::resources::Profile::BoundedV1,
        ) => Ok(PROFILE_70450_AVAILABLE_GEOMETRY),
        (
            super::target::Target::Build70450,
            plan::MapSelection::All,
            run::MissingWdtPolicy::DeclareRootAbsent,
            run::MissingModelPolicy::Reject,
            run::MissingWmoPolicy::Reject,
            crate::collision::resources::Profile::BoundedV1,
        ) => Ok(PROFILE_70450_AVAILABLE_ROOT_ABSENT_GEOMETRY),
        (
            super::target::Target::Build70450,
            plan::MapSelection::All,
            run::MissingWdtPolicy::DeclareRootAbsent,
            run::MissingModelPolicy::DeclareRootAbsentWmoDoodadM2,
            run::MissingWmoPolicy::Reject,
            crate::collision::resources::Profile::BoundedV1,
        ) => Ok(PROFILE_70450_AVAILABLE_MODEL_ROOT_ABSENT_GEOMETRY),
        (
            super::target::Target::Build70450,
            plan::MapSelection::All,
            run::MissingWdtPolicy::DeclareRootAbsent,
            run::MissingModelPolicy::DeclareRootAbsentWmoDoodadM2,
            run::MissingWmoPolicy::Reject,
            crate::collision::resources::Profile::RequestHeavyV1,
        ) => Ok(PROFILE_70450_AVAILABLE_MODEL_ROOT_ABSENT_REQUEST_CAPACITY_GEOMETRY),
        (
            super::target::Target::Build70450,
            plan::MapSelection::All,
            run::MissingWdtPolicy::DeclareRootAbsent,
            run::MissingModelPolicy::DeclareRootAbsentWmoDoodadM2,
            run::MissingWmoPolicy::DeclareVacatedAfterRootAbsentM2,
            crate::collision::resources::Profile::RequestHeavyV1,
        ) => Ok(PROFILE_70450_AVAILABLE_VACATED_WMO_GEOMETRY),
        // Only v34 admits intrinsic emptiness; v33 retains its separate policy and profile.
        (
            super::target::Target::Build70450,
            plan::MapSelection::All,
            run::MissingWdtPolicy::DeclareRootAbsent,
            run::MissingModelPolicy::DeclareRootAbsentWmoDoodadM2,
            run::MissingWmoPolicy::DeclareVacatedAfterRootAbsentM2OrEmpty,
            crate::collision::resources::Profile::RequestHeavyV1,
        ) => Ok(PROFILE_70450_AVAILABLE_EMPTY_WMO_GEOMETRY),
        (
            super::target::Target::Build70450,
            plan::MapSelection::All,
            run::MissingWdtPolicy::DeclareRootAbsent,
            run::MissingModelPolicy::DeclareRootAbsentWmoDoodadM2,
            run::MissingWmoPolicy::DeclareVacatedAfterRootAbsentM2OrEmptyOrUnselected,
            crate::collision::resources::Profile::RequestHeavyV1,
        ) => Ok(PROFILE_70450_AVAILABLE_UNSELECTED_WMO_GEOMETRY),
        (
            super::target::Target::Build70450,
            plan::MapSelection::All,
            run::MissingWdtPolicy::DeclareRootAbsent,
            run::MissingModelPolicy::DeclareRootAbsentWmoDoodadM2,
            run::MissingWmoPolicy::DeclareVacatedAfterRootAbsentM2OrEmptyOrUnselectedOrMaterialFiltered,
            crate::collision::resources::Profile::RequestHeavyV1,
        ) => Ok(PROFILE_70450_AVAILABLE_MATERIAL_FILTERED_WMO_GEOMETRY),
        (
            super::target::Target::Build70450,
            plan::MapSelection::All,
            run::MissingWdtPolicy::DeclareRootAbsent,
            run::MissingModelPolicy::DeclareRootAbsentWmoDoodadM2,
            run::MissingWmoPolicy::DeclareVacatedAfterRootAbsentM2OrEmptyOrUnselectedOrMaterialFiltered,
            crate::collision::resources::Profile::DependencyHeavyV1,
        ) => Ok(PROFILE_70450_AVAILABLE_DEPENDENCY_HEAVY_MATERIAL_FILTERED_WMO_GEOMETRY),
        (
            super::target::Target::Build70450,
            plan::MapSelection::All,
            run::MissingWdtPolicy::DeclareRootAbsent,
            run::MissingModelPolicy::DeclareRootAbsentWmoDoodadM2,
            run::MissingWmoPolicy::DeclareVacatedAfterRootAbsentM2OrEmptyOrUnselectedOrMaterialFiltered,
            crate::collision::resources::Profile::SourceHeavyV1,
        ) => Ok(PROFILE_70450_AVAILABLE_SOURCE_HEAVY_MATERIAL_FILTERED_WMO_GEOMETRY),
        (
            super::target::Target::Build70450,
            plan::MapSelection::All,
            run::MissingWdtPolicy::DeclareRootAbsent,
            run::MissingModelPolicy::DeclareRootAbsentWmoDoodadM2,
            run::MissingWmoPolicy::DeclareVacatedAfterRootAbsentM2OrEmptyOrUnselectedOrMaterialFiltered,
            crate::collision::resources::Profile::ReadHeavyV1,
        ) => Ok(PROFILE_70450_AVAILABLE_READ_HEAVY_MATERIAL_FILTERED_WMO_GEOMETRY),
        (
            super::target::Target::Build70450,
            plan::MapSelection::All,
            run::MissingWdtPolicy::DeclareRootAbsent,
            run::MissingModelPolicy::DeclareRootAbsentWmoDoodadM2,
            run::MissingWmoPolicy::DeclareVacatedAfterRootAbsentM2OrEmptyOrUnselectedOrMaterialFiltered,
            crate::collision::resources::Profile::ModelHeavyV1,
        ) => Ok(PROFILE_70450_AVAILABLE_MODEL_HEAVY_MATERIAL_FILTERED_WMO_GEOMETRY),
        (
            super::target::Target::Build70450,
            plan::MapSelection::All,
            run::MissingWdtPolicy::DeclareRootAbsent,
            run::MissingModelPolicy::DeclareRootAbsentWmoDoodadM2,
            run::MissingWmoPolicy::DeclareVacatedAfterRootAbsentM2OrEmptyOrUnselectedOrMaterialFiltered,
            crate::collision::resources::Profile::SourceExpandedV1,
        ) => Ok(PROFILE_70450_AVAILABLE_SOURCE_EXPANDED_MATERIAL_FILTERED_WMO_GEOMETRY),
        (
            super::target::Target::Build70450,
            plan::MapSelection::All,
            run::MissingWdtPolicy::DeclareRootAbsent,
            run::MissingModelPolicy::DeclareRootAbsentWmoDoodadM2,
            run::MissingWmoPolicy::DeclareVacatedAfterRootAbsentM2OrEmptyOrUnselectedOrMaterialFiltered,
            crate::collision::resources::Profile::SourceExpandedV2,
        ) => Ok(PROFILE_70450_AVAILABLE_SOURCE_EXPANDED_V2_MATERIAL_FILTERED_WMO_GEOMETRY),
        (
            super::target::Target::Build70450,
            plan::MapSelection::All,
            run::MissingWdtPolicy::DeclareRootAbsent,
            run::MissingModelPolicy::DeclareRootAbsentWmoDoodadM2,
            run::MissingWmoPolicy::DeclareVacatedAfterRootAbsentM2OrEmptyOrUnselectedOrMaterialFiltered,
            crate::collision::resources::Profile::ModelOutputExpandedV1,
        ) => Ok(PROFILE_70450_AVAILABLE_MODEL_OUTPUT_EXPANDED_MATERIAL_FILTERED_WMO_GEOMETRY),
        (
            super::target::Target::Build70450,
            plan::MapSelection::All,
            run::MissingWdtPolicy::DeclareRootAbsent,
            run::MissingModelPolicy::DeclareRootAbsentWmoDoodadM2,
            run::MissingWmoPolicy::DeclareVacatedAfterRootAbsentM2OrEmptyOrUnselectedOrMaterialFiltered,
            crate::collision::resources::Profile::ModelBytesExpandedV1,
        ) => Ok(PROFILE_70450_AVAILABLE_MODEL_BYTES_EXPANDED_MATERIAL_FILTERED_WMO_GEOMETRY),
        (
            super::target::Target::Build70450,
            plan::MapSelection::All,
            run::MissingWdtPolicy::DeclareRootAbsent,
            run::MissingModelPolicy::DeclareRootAbsentWmoDoodadM2,
            run::MissingWmoPolicy::DeclareVacatedAfterRootAbsentM2OrEmptyOrUnselectedOrMaterialFiltered,
            crate::collision::resources::Profile::PlacementExpandedV1,
        ) => Ok(PROFILE_70450_AVAILABLE_PLACEMENT_EXPANDED_MATERIAL_FILTERED_WMO_GEOMETRY),
        (
            super::target::Target::Build70450,
            plan::MapSelection::All,
            run::MissingWdtPolicy::DeclareRootAbsent,
            run::MissingModelPolicy::DeclareRootAbsentWmoDoodadM2,
            run::MissingWmoPolicy::DeclareVacatedAfterRootAbsentM2OrEmptyOrUnselectedOrMaterialFiltered,
            crate::collision::resources::Profile::AssemblyExpandedV1,
        ) => Ok(PROFILE_70450_AVAILABLE_ASSEMBLY_EXPANDED_MATERIAL_FILTERED_WMO_GEOMETRY),
        (
            super::target::Target::Build70450,
            plan::MapSelection::All,
            run::MissingWdtPolicy::DeclareRootAbsent,
            run::MissingModelPolicy::DeclareRootAbsentWmoDoodadM2,
            run::MissingWmoPolicy::DeclareVacatedAfterRootAbsentM2OrEmptyOrUnselectedOrMaterialFiltered,
            crate::collision::resources::Profile::GameObjectExpandedV1,
        ) => Ok(PROFILE_70450_AVAILABLE_GAMEOBJECT_EXPANDED_MATERIAL_FILTERED_WMO_GEOMETRY),
        (
            super::target::Target::Build70450,
            plan::MapSelection::All,
            run::MissingWdtPolicy::DeclareRootAbsent,
            run::MissingModelPolicy::DeclareRootAbsentWmoDoodadM2,
            run::MissingWmoPolicy::DeclareVacatedAfterRootAbsentM2OrEmptyOrUnselectedOrMaterialFiltered,
            crate::collision::resources::Profile::GameObjectMissingRootExpandedV1,
        ) => Ok(PROFILE_70450_AVAILABLE_GAMEOBJECT_MISSING_ROOT_MATERIAL_FILTERED_WMO_GEOMETRY),
        (
            super::target::Target::Build70450,
            plan::MapSelection::All,
            run::MissingWdtPolicy::DeclareRootAbsent,
            run::MissingModelPolicy::DeclareRootAbsentWmoDoodadM2,
            run::MissingWmoPolicy::DeclareVacatedAfterRootAbsentM2OrEmptyOrUnselectedOrMaterialFiltered,
            crate::collision::resources::Profile::GameObjectRootAndEncryptedUnavailableExpandedV1,
        ) => Ok(PROFILE_70450_AVAILABLE_GAMEOBJECT_ROOT_ENCRYPTED_UNAVAILABLE_MATERIAL_FILTERED_WMO_GEOMETRY),
        (
            super::target::Target::Build70450,
            plan::MapSelection::MapAndAncestors(_),
            run::MissingWdtPolicy::Reject,
            run::MissingModelPolicy::Reject,
            run::MissingWmoPolicy::Reject,
            crate::collision::resources::Profile::BoundedV1,
        ) => Ok(PROFILE_70450_SELECTED_GEOMETRY),
        // No profile silently inherits 70374. Add an explicit target/scope pair
        // only when its complete reader and completion gate are implemented.
        _ => Err(Error::State),
    }
}

fn extract_with_selection(
    startup: Startup<'_>,
    sink: &mut impl run::Sink,
    limits: world::Limits,
    target: super::target::Target,
    selection: plan::MapSelection,
) -> Result<Completed, Error> {
    extract_with_policy(
        startup,
        sink,
        limits,
        target,
        selection,
        run::MissingWdtPolicy::Reject,
    )
}

#[derive(Clone, Copy)]
struct OmissionOptions {
    missing_wdt: run::MissingWdtPolicy,
    missing_model: run::MissingModelPolicy,
    missing_wmo: run::MissingWmoPolicy,
    resource: crate::collision::resources::Profile,
}

fn extract_with_policy(
    startup: Startup<'_>,
    sink: &mut impl run::Sink,
    limits: world::Limits,
    target: super::target::Target,
    selection: plan::MapSelection,
    missing_wdt: run::MissingWdtPolicy,
) -> Result<Completed, Error> {
    extract_with_omissions(
        startup,
        sink,
        limits,
        target,
        selection,
        OmissionOptions {
            missing_wdt,
            missing_model: run::MissingModelPolicy::Reject,
            missing_wmo: run::MissingWmoPolicy::Reject,
            resource: crate::collision::resources::Profile::BoundedV1,
        },
    )
}

fn extract_with_omissions(
    mut startup: Startup<'_>,
    sink: &mut impl run::Sink,
    limits: world::Limits,
    target: super::target::Target,
    selection: plan::MapSelection,
    options: OmissionOptions,
) -> Result<Completed, Error> {
    let OmissionOptions {
        missing_wdt,
        missing_model,
        missing_wmo,
        resource,
    } = options;
    let profile = completion_profile_with_omissions(
        target,
        selection,
        missing_wdt,
        missing_model,
        missing_wmo,
        resource,
    )?;
    let godi_startup_reads = startup.data().startup_facts().reads;
    let result = (|| {
        limits.validate_resources(resource).map_err(Error::World)?;
        prepare_available_target(startup.collector_mut(), limits, target).map_err(|e| match e {
            PrepareFailure::Available(e) => e,
            PrepareFailure::Collector(e) => Error::Collision(e),
        })
    })();
    let Prepared {
        owner,
        mut map,
        lookup,
        liquids,
        retained,
    } = match result {
        Ok(value) => value,
        Err(e) => {
            startup.collector_mut().abort();
            return Err(e);
        }
    };
    let prepared_graph = (|| {
        let graph_budget = limits.maps.max_memory_bytes.min(
            limits
                .max_table_memory_bytes
                .checked_sub(retained)
                .ok_or(Error::Limit)?,
        );
        startup
            .collector_mut()
            .reserve_external(graph_budget)
            .map_err(Error::Collision)?;
        let maps = plan::Maps::from_materialized_selection(
            owner.rows(),
            owner.direct_records().len(),
            selection,
            plan::Limits {
                max_memory_bytes: graph_budget,
                ..limits.maps
            },
        )
        .map_err(Error::Plan)?;
        let resident = maps.stats().resident_bytes;
        if resident > graph_budget || add(retained, resident)? > limits.max_table_memory_bytes {
            return Err(Error::Limit);
        }
        startup
            .collector_mut()
            .release_external(graph_budget - resident)
            .map_err(Error::Collision)?;
        let remaining = limits
            .max_table_memory_bytes
            .checked_sub(add(retained, resident)?)
            .ok_or(Error::Limit)?;
        let topology = world::Topology::snapshot(&maps, startup.collector_mut(), remaining)
            .map_err(Error::World)?;
        Ok((maps, topology))
    })();
    let (maps, topology) = match prepared_graph {
        Ok(value) => value,
        Err(e) => {
            startup.collector_mut().abort();
            return Err(e);
        }
    };
    let map_stats = maps.stats();
    let lookup_stats = world::LookupStats {
        materials: lookup.materials.len(),
        types: lookup.types.len(),
        retained_bytes: add(capacity(&lookup.materials)?, capacity(&lookup.types)?)?,
    };
    let mut context = startup.with_lookup(&lookup)?;
    let (
        report,
        root_absent,
        missing_m2,
        missing_wmo,
        empty_wmo,
        unselected_wmo,
        material_filtered_wmo,
    ) = match missing_wmo {
            run::MissingWmoPolicy::Reject => {
                let (report, root_absent, missing_m2) =
                    run::shared::extract_collector_with_omissions(
                        context.collector_mut(),
                        &maps,
                        sink,
                        limits.run,
                        missing_wdt,
                        missing_model,
                    )
                    .map_err(Error::Run)?;
                (
                    report,
                    root_absent,
                    missing_m2,
                    Vec::new(),
                    Vec::new(),
                    Vec::new(),
                    Vec::new(),
                )
            }
            run::MissingWmoPolicy::DeclareVacatedAfterRootAbsentM2 => {
                let (report, root_absent, missing_m2, missing_wmo, empty_wmo) =
                    run::shared::extract_collector_with_vacated_wmo(
                        context.collector_mut(),
                        &maps,
                        sink,
                        limits.run,
                        missing_wdt,
                        missing_model,
                    )
                    .map_err(Error::Run)?;
                (
                    report,
                    root_absent,
                    missing_m2,
                    missing_wmo,
                    empty_wmo,
                    Vec::new(),
                    Vec::new(),
                )
            }
            run::MissingWmoPolicy::DeclareVacatedAfterRootAbsentM2OrEmpty => {
                let (report, root_absent, missing_m2, missing_wmo, empty_wmo) =
                    run::shared::extract_collector_with_empty_wmo(
                        context.collector_mut(),
                        &maps,
                        sink,
                        limits.run,
                        missing_wdt,
                        missing_model,
                    )
                    .map_err(Error::Run)?;
                (
                    report,
                    root_absent,
                    missing_m2,
                    missing_wmo,
                    empty_wmo,
                    Vec::new(),
                    Vec::new(),
                )
            }
            run::MissingWmoPolicy::DeclareVacatedAfterRootAbsentM2OrEmptyOrUnselected => {
                let (report, root_absent, missing_m2, missing_wmo, empty_wmo, unselected_wmo) =
                    run::shared::extract_collector_with_unselected_wmo(
                        context.collector_mut(),
                        &maps,
                        sink,
                        limits.run,
                        missing_wdt,
                        missing_model,
                    )
                    .map_err(Error::Run)?;
                (
                    report,
                    root_absent,
                    missing_m2,
                    missing_wmo,
                    empty_wmo,
                    unselected_wmo,
                    Vec::new(),
                )
            }
            run::MissingWmoPolicy::DeclareVacatedAfterRootAbsentM2OrEmptyOrUnselectedOrMaterialFiltered => {
                run::shared::extract_collector_with_material_filtered_wmo(
                    context.collector_mut(),
                    &maps,
                    sink,
                    limits.run,
                    missing_wdt,
                    missing_model,
                )
                .map_err(Error::Run)?
            }
        };
    let root_absent_bytes = capacity(&root_absent)?;
    let missing_m2_bytes = capacity(&missing_m2)?;
    let missing_wmo_bytes = capacity(&missing_wmo)?;
    let empty_wmo_bytes = capacity(&empty_wmo)?;
    let unselected_wmo_bytes = capacity(&unselected_wmo)?;
    let material_filtered_wmo_bytes = capacity(&material_filtered_wmo)?;
    let mut context = context.with_lookup(&EMPTY)?;
    drop(maps);
    drop(owner);
    drop(lookup);
    context
        .collector_mut()
        .release_external(add(retained, map_stats.resident_bytes)?)
        .map_err(Error::Collision)?;
    context
        .collector_mut()
        .release_external(topology.resident_bytes())
        .map_err(Error::Collision)?;
    let startup = context.finish()?;
    let resident_bytes = add(
        add(
            startup.extraction().resident_bytes(),
            startup.data().stats().resident_bytes,
        )?,
        topology.resident_bytes(),
    )?;
    let resident_bytes = add(resident_bytes, root_absent_bytes)?;
    let resident_bytes = add(resident_bytes, missing_m2_bytes)?;
    let resident_bytes = add(resident_bytes, missing_wmo_bytes)?;
    let resident_bytes = add(resident_bytes, empty_wmo_bytes)?;
    let resident_bytes = add(resident_bytes, unselected_wmo_bytes)?;
    let resident_bytes = add(resident_bytes, material_filtered_wmo_bytes)?;
    map.materialized_rows = map_stats.rows;
    Ok(Completed {
        profile,
        startup,
        report,
        root_absent,
        missing_m2,
        missing_wmo,
        empty_wmo,
        unselected_wmo,
        material_filtered_wmo,
        map,
        liquids,
        lookups: lookup_stats,
        maps: map_stats,
        topology: Topology { inner: topology },
        resident_bytes,
        startup_reads: godi_startup_reads,
    })
}
pub(in crate::collision) struct Prepared {
    pub(in crate::collision) owner: db2::MaterializedVMapMaps,
    pub(in crate::collision) map: TableFacts,
    pub(in crate::collision) lookup: world::Lookups,
    pub(in crate::collision) liquids: [world::TableFacts; 2],
    pub(in crate::collision) retained: usize,
}
// Private shared Map preparation. Available facts stay distinct; complete
// liquid reads still use the established world::input::Access contract.
pub(in crate::collision) enum PrepareFailure<E> {
    Available(Error),
    Collector(E),
}
impl<E> From<Error> for PrepareFailure<E> {
    fn from(e: Error) -> Self {
        Self::Available(e)
    }
}
pub(in crate::collision) trait AvailableAccess: world::input::Access {
    fn read_available_source_for_target(
        &mut self,
        id: u32,
        cap: usize,
        target: super::target::Target,
    ) -> Result<acquired::AvailableSource, Self::Error>;
}
impl<L: LiquidLookup + ?Sized> AvailableAccess for Extractor<'_, '_, L> {
    fn read_available_source_for_target(
        &mut self,
        id: u32,
        cap: usize,
        target: super::target::Target,
    ) -> Result<acquired::AvailableSource, Self::Error> {
        Extractor::read_available_source_target(self, id, cap, target)
    }
}
impl<L: LiquidLookup + ?Sized> AvailableAccess for super::payload::SpoolingExtractor<'_, '_, L> {
    fn read_available_source_for_target(
        &mut self,
        id: u32,
        cap: usize,
        target: super::target::Target,
    ) -> Result<acquired::AvailableSource, Self::Error> {
        let result = self.inner.read_available_source_target(id, cap, target);
        self.checked(result)
    }
}
fn prepare_available<A: AvailableAccess>(
    collector: &mut A,
    limits: world::Limits,
) -> Result<Prepared, PrepareFailure<A::Error>> {
    prepare_available_target(collector, limits, super::target::Target::Build70374)
}

pub(in crate::collision) fn prepare_available_target<A: AvailableAccess>(
    collector: &mut A,
    limits: world::Limits,
    target: super::target::Target,
) -> Result<Prepared, PrepareFailure<A::Error>> {
    let file = collector
        .read_available_source_for_target(world::MAP_FILE_DATA_ID, limits.max_db2_bytes, target)
        .map_err(PrepareFailure::Collector)?;
    let source_resident = file
        .resident_bytes()
        .map_err(|e| Error::Collision(super::Error::from_available(e)))?;
    header_policy(&file, limits.max_direct_rows, limits.max_copy_records)?;
    let allowance = allowance(
        file.declared_bytes(),
        true,
        limits.max_decode_allowance_bytes,
    )?;
    collector
        .reserve_external(allowance)
        .map_err(PrepareFailure::Collector)?;
    let snapshot = match target {
        super::target::Target::Build70374 => db2::read_vmap_maps_70374(&file),
        super::target::Target::Build70425 => db2::read_vmap_maps_70425(&file),
        super::target::Target::Build70450 => db2::read_vmap_maps_70450(&file),
    }
    .map_err(Error::Db2)?;
    let mut map = table_facts(
        &file,
        snapshot.sections(),
        snapshot.direct_records().len(),
        snapshot.copies().len(),
        allowance,
    )?;
    if map.declared_direct_rows > limits.max_direct_rows
        || map.declared_copy_records > limits.max_copy_records
    {
        return Err(Error::Limit.into());
    }
    for row in snapshot.direct_records() {
        if row.fields.name.len() > limits.maps.max_name_bytes
            || row.fields.directory.len() > limits.maps.max_name_bytes
        {
            return Err(Error::Limit.into());
        }
    }
    let owner = snapshot.materialize().map_err(Error::Db2)?;
    let retained = owner.resident_bytes().map_err(Error::Db2)?;
    if owner.rows().len() > limits.maps.max_rows
        || retained > limits.max_table_memory_bytes
        || retained > allowance
    {
        return Err(Error::Limit.into());
    }
    map.materialized_rows = owner.rows().len();
    map.retained_bytes = retained;
    drop(file);
    collector
        .release_source(source_resident)
        .map_err(PrepareFailure::Collector)?;
    collector
        .release_external(allowance - retained)
        .map_err(PrepareFailure::Collector)?;
    let (lookup, liquids, retained) =
        world::input::prepare_liquids_target(collector, limits, retained, target).map_err(|e| {
            match e {
                world::input::Failure::World(e) => PrepareFailure::Available(Error::World(e)),
                world::input::Failure::Collector(e) => PrepareFailure::Collector(e),
            }
        })?;
    Ok(Prepared {
        owner,
        map,
        lookup,
        liquids,
        retained,
    })
}
pub(in crate::collision) fn allowance(bytes: u64, text: bool, cap: usize) -> Result<usize, Error> {
    let value = usize::try_from(bytes)
        .map_err(|_| Error::Limit)?
        .checked_mul(32)
        .and_then(|n| n.checked_add(2 * 1024 * 1024))
        .and_then(|n| n.checked_add(if text { 64 * 1024 * 1024 } else { 0 }))
        .ok_or(Error::Limit)?;
    if value > cap {
        return Err(Error::Limit);
    }
    Ok(value)
}
pub(in crate::collision) fn table_facts(
    file: &acquired::AvailableSource,
    sections: &[db2::SectionCoverage],
    direct_rows: usize,
    copy_records: usize,
    allowance: usize,
) -> Result<TableFacts, Error> {
    if sections.len() > 16 {
        return Err(Error::Limit);
    }
    let mut slots = [None; 16];
    let mut rows = 0;
    let mut copies = 0;
    for (slot, s) in slots.iter_mut().zip(sections) {
        rows = add(rows, s.declared_records as usize)?;
        copies = add(copies, s.declared_copies as usize)?;
        *slot = Some(*s);
    }
    Ok(TableFacts {
        file_data_id: file.file_data_id(),
        declared_bytes: file.declared_bytes(),
        available_bytes: file.available_bytes(),
        full_ckey_verified: file.full_ckey_verified(),
        encoded_key_authenticated: file.encoded_key_authenticated(),
        work: file.work(),
        declared_direct_rows: rows,
        declared_copy_records: copies,
        direct_rows,
        copy_records,
        materialized_rows: 0,
        retained_bytes: 0,
        decode_allowance_bytes: allowance,
        sections: slots,
    })
}
pub(in crate::collision) fn add(a: usize, b: usize) -> Result<usize, Error> {
    a.checked_add(b).ok_or(Error::Limit)
}
pub(in crate::collision) fn capacity<T>(rows: &Vec<T>) -> Result<usize, Error> {
    rows.capacity()
        .checked_mul(std::mem::size_of::<T>())
        .ok_or(Error::Limit)
}

// Early downward policy admission only; the decoder still validates the whole
// header, column metadata, known section/text extents and declared relationships.
pub(in crate::collision) fn header_policy(
    file: &acquired::AvailableSource,
    max_rows: usize,
    max_copies: usize,
) -> Result<(), Error> {
    let word = |at| -> Result<usize, Error> {
        let bytes = file
            .range(at, 4)
            .map_err(|_| Error::Db2(RecordError::Bounds))?;
        Ok(u32::from_le_bytes(
            bytes
                .try_into()
                .map_err(|_| Error::Db2(RecordError::Bounds))?,
        ) as usize)
    };
    if word(136)? > max_rows {
        return Err(Error::Limit);
    }
    let count = word(200)?;
    if count > 16 {
        return Err(Error::Limit);
    }
    let mut copies = 0;
    for i in 0..count {
        copies = add(copies, word((240 + 40 * i) as u64)?)?;
    }
    if copies > max_copies {
        return Err(Error::Limit);
    }
    Ok(())
}

#[cfg(test)]
mod profile_tests {
    use super::*;

    #[test]
    fn placement_expanded_completion_requires_exact_v45_scope_and_omissions() {
        use crate::collision::resources::Profile;
        use crate::collision::target::Target;

        let material_filtered =
            run::MissingWmoPolicy::DeclareVacatedAfterRootAbsentM2OrEmptyOrUnselectedOrMaterialFiltered;
        assert_eq!(
            completion_profile_with_omissions(
                Target::Build70450,
                plan::MapSelection::All,
                run::MissingWdtPolicy::DeclareRootAbsent,
                run::MissingModelPolicy::DeclareRootAbsentWmoDoodadM2,
                material_filtered,
                Profile::PlacementExpandedV1,
            ),
            Ok(PROFILE_70450_AVAILABLE_PLACEMENT_EXPANDED_MATERIAL_FILTERED_WMO_GEOMETRY)
        );
        for (target, selection, missing_wdt, missing_model, missing_wmo) in [
            (
                Target::Build70425,
                plan::MapSelection::All,
                run::MissingWdtPolicy::DeclareRootAbsent,
                run::MissingModelPolicy::DeclareRootAbsentWmoDoodadM2,
                material_filtered,
            ),
            (
                Target::Build70450,
                plan::MapSelection::MapAndAncestors(7),
                run::MissingWdtPolicy::DeclareRootAbsent,
                run::MissingModelPolicy::DeclareRootAbsentWmoDoodadM2,
                material_filtered,
            ),
            (
                Target::Build70450,
                plan::MapSelection::All,
                run::MissingWdtPolicy::Reject,
                run::MissingModelPolicy::DeclareRootAbsentWmoDoodadM2,
                material_filtered,
            ),
            (
                Target::Build70450,
                plan::MapSelection::All,
                run::MissingWdtPolicy::DeclareRootAbsent,
                run::MissingModelPolicy::Reject,
                material_filtered,
            ),
            (
                Target::Build70450,
                plan::MapSelection::All,
                run::MissingWdtPolicy::DeclareRootAbsent,
                run::MissingModelPolicy::DeclareRootAbsentWmoDoodadM2,
                run::MissingWmoPolicy::Reject,
            ),
        ] {
            assert_eq!(
                completion_profile_with_omissions(
                    target,
                    selection,
                    missing_wdt,
                    missing_model,
                    missing_wmo,
                    Profile::PlacementExpandedV1,
                ),
                Err(Error::State)
            );
        }
        assert_eq!(
            completion_profile_with_omissions(
                Target::Build70450,
                plan::MapSelection::All,
                run::MissingWdtPolicy::DeclareRootAbsent,
                run::MissingModelPolicy::DeclareRootAbsentWmoDoodadM2,
                material_filtered,
                Profile::DiskWorldV1,
            ),
            Err(Error::State)
        );
    }

    #[test]
    fn assembly_expanded_completion_has_its_own_v46_identity() {
        use crate::collision::resources::Profile;
        use crate::collision::target::Target;

        assert_eq!(
            completion_profile_with_omissions(
                Target::Build70450,
                plan::MapSelection::All,
                run::MissingWdtPolicy::DeclareRootAbsent,
                run::MissingModelPolicy::DeclareRootAbsentWmoDoodadM2,
                run::MissingWmoPolicy::DeclareVacatedAfterRootAbsentM2OrEmptyOrUnselectedOrMaterialFiltered,
                Profile::AssemblyExpandedV1,
            ),
            Ok(PROFILE_70450_AVAILABLE_ASSEMBLY_EXPANDED_MATERIAL_FILTERED_WMO_GEOMETRY)
        );
    }

    #[test]
    fn target_and_selection_require_an_explicit_completion_profile() {
        assert_eq!(
            completion_profile(
                crate::collision::target::Target::Build70374,
                plan::MapSelection::All
            ),
            Ok(PROFILE)
        );
        assert_eq!(
            completion_profile(
                crate::collision::target::Target::Build70450,
                plan::MapSelection::MapAndAncestors(2)
            ),
            Ok(PROFILE_70450_SELECTED_GEOMETRY)
        );
        assert_eq!(
            completion_profile(
                crate::collision::target::Target::Build70425,
                plan::MapSelection::All
            ),
            Err(Error::State)
        );
    }

    #[test]
    fn completion_profile_follows_missing_wdt_policy_not_map_selection() {
        assert_eq!(
            completion_profile_with_policy(
                crate::collision::target::Target::Build70450,
                plan::MapSelection::All,
                run::MissingWdtPolicy::Reject,
            ),
            Ok(PROFILE_70450_AVAILABLE_GEOMETRY)
        );
        assert_eq!(
            completion_profile_with_policy(
                crate::collision::target::Target::Build70450,
                plan::MapSelection::All,
                run::MissingWdtPolicy::DeclareRootAbsent,
            ),
            Ok(PROFILE_70450_AVAILABLE_ROOT_ABSENT_GEOMETRY)
        );
        assert_eq!(
            completion_profile_with_policy(
                crate::collision::target::Target::Build70450,
                plan::MapSelection::MapAndAncestors(2),
                run::MissingWdtPolicy::DeclareRootAbsent,
            ),
            Err(Error::State)
        );
    }

    #[test]
    fn model_omission_profile_requires_both_explicit_policies() {
        assert_eq!(
            completion_profile_with_omissions(
                crate::collision::target::Target::Build70450,
                plan::MapSelection::All,
                run::MissingWdtPolicy::DeclareRootAbsent,
                run::MissingModelPolicy::DeclareRootAbsentWmoDoodadM2,
                run::MissingWmoPolicy::Reject,
                crate::collision::resources::Profile::BoundedV1,
            ),
            Ok(PROFILE_70450_AVAILABLE_MODEL_ROOT_ABSENT_GEOMETRY)
        );
        assert_eq!(
            completion_profile_with_omissions(
                crate::collision::target::Target::Build70450,
                plan::MapSelection::All,
                run::MissingWdtPolicy::DeclareRootAbsent,
                run::MissingModelPolicy::Reject,
                run::MissingWmoPolicy::Reject,
                crate::collision::resources::Profile::BoundedV1,
            ),
            Ok(PROFILE_70450_AVAILABLE_ROOT_ABSENT_GEOMETRY)
        );
        assert_eq!(
            completion_profile_with_omissions(
                crate::collision::target::Target::Build70450,
                plan::MapSelection::All,
                run::MissingWdtPolicy::Reject,
                run::MissingModelPolicy::DeclareRootAbsentWmoDoodadM2,
                run::MissingWmoPolicy::Reject,
                crate::collision::resources::Profile::BoundedV1,
            ),
            Err(Error::State)
        );
    }

    #[test]
    fn request_capacity_profile_requires_exact_target_policies_and_resource() {
        assert_eq!(
            completion_profile_with_omissions(
                crate::collision::target::Target::Build70450,
                plan::MapSelection::All,
                run::MissingWdtPolicy::DeclareRootAbsent,
                run::MissingModelPolicy::DeclareRootAbsentWmoDoodadM2,
                run::MissingWmoPolicy::Reject,
                crate::collision::resources::Profile::RequestHeavyV1,
            ),
            Ok(PROFILE_70450_AVAILABLE_MODEL_ROOT_ABSENT_REQUEST_CAPACITY_GEOMETRY)
        );
        for resource in [
            crate::collision::resources::Profile::BoundedV1,
            crate::collision::resources::Profile::DiskWorldV1,
        ] {
            assert_eq!(
                completion_profile_with_omissions(
                    crate::collision::target::Target::Build70450,
                    plan::MapSelection::All,
                    run::MissingWdtPolicy::DeclareRootAbsent,
                    run::MissingModelPolicy::DeclareRootAbsentWmoDoodadM2,
                    run::MissingWmoPolicy::Reject,
                    resource,
                ),
                if resource == crate::collision::resources::Profile::BoundedV1 {
                    Ok(PROFILE_70450_AVAILABLE_MODEL_ROOT_ABSENT_GEOMETRY)
                } else {
                    Err(Error::State)
                }
            );
        }
        assert_eq!(
            completion_profile_with_omissions(
                crate::collision::target::Target::Build70450,
                plan::MapSelection::All,
                run::MissingWdtPolicy::Reject,
                run::MissingModelPolicy::DeclareRootAbsentWmoDoodadM2,
                run::MissingWmoPolicy::Reject,
                crate::collision::resources::Profile::RequestHeavyV1,
            ),
            Err(Error::State)
        );
    }

    #[test]
    fn model_output_expanded_completion_requires_exact_build_scope_and_omissions() {
        use crate::collision::resources::Profile;
        use crate::collision::target::Target;

        let material_filtered =
            run::MissingWmoPolicy::DeclareVacatedAfterRootAbsentM2OrEmptyOrUnselectedOrMaterialFiltered;
        assert_eq!(
            completion_profile_with_omissions(
                Target::Build70450,
                plan::MapSelection::All,
                run::MissingWdtPolicy::DeclareRootAbsent,
                run::MissingModelPolicy::DeclareRootAbsentWmoDoodadM2,
                material_filtered,
                Profile::ModelOutputExpandedV1,
            ),
            Ok(PROFILE_70450_AVAILABLE_MODEL_OUTPUT_EXPANDED_MATERIAL_FILTERED_WMO_GEOMETRY)
        );
        assert_eq!(
            completion_profile_with_omissions(
                Target::Build70450,
                plan::MapSelection::All,
                run::MissingWdtPolicy::DeclareRootAbsent,
                run::MissingModelPolicy::DeclareRootAbsentWmoDoodadM2,
                material_filtered,
                Profile::SourceExpandedV1,
            ),
            Ok(PROFILE_70450_AVAILABLE_SOURCE_EXPANDED_MATERIAL_FILTERED_WMO_GEOMETRY)
        );
        for (target, selection, missing_wdt, missing_model, missing_wmo, resource) in [
            (
                Target::Build70425,
                plan::MapSelection::All,
                run::MissingWdtPolicy::DeclareRootAbsent,
                run::MissingModelPolicy::DeclareRootAbsentWmoDoodadM2,
                material_filtered,
                Profile::ModelOutputExpandedV1,
            ),
            (
                Target::Build70450,
                plan::MapSelection::MapAndAncestors(7),
                run::MissingWdtPolicy::DeclareRootAbsent,
                run::MissingModelPolicy::DeclareRootAbsentWmoDoodadM2,
                material_filtered,
                Profile::ModelOutputExpandedV1,
            ),
            (
                Target::Build70450,
                plan::MapSelection::All,
                run::MissingWdtPolicy::Reject,
                run::MissingModelPolicy::DeclareRootAbsentWmoDoodadM2,
                material_filtered,
                Profile::ModelOutputExpandedV1,
            ),
            (
                Target::Build70450,
                plan::MapSelection::All,
                run::MissingWdtPolicy::DeclareRootAbsent,
                run::MissingModelPolicy::Reject,
                material_filtered,
                Profile::ModelOutputExpandedV1,
            ),
            (
                Target::Build70450,
                plan::MapSelection::All,
                run::MissingWdtPolicy::DeclareRootAbsent,
                run::MissingModelPolicy::DeclareRootAbsentWmoDoodadM2,
                run::MissingWmoPolicy::Reject,
                Profile::ModelOutputExpandedV1,
            ),
            (
                Target::Build70450,
                plan::MapSelection::All,
                run::MissingWdtPolicy::DeclareRootAbsent,
                run::MissingModelPolicy::DeclareRootAbsentWmoDoodadM2,
                material_filtered,
                Profile::DiskWorldV1,
            ),
        ] {
            assert_eq!(
                completion_profile_with_omissions(
                    target,
                    selection,
                    missing_wdt,
                    missing_model,
                    missing_wmo,
                    resource,
                ),
                Err(Error::State)
            );
        }
    }

    #[test]
    fn source_expanded_completion_requires_exact_build_scope_and_omissions() {
        use crate::collision::resources::Profile;
        use crate::collision::target::Target;

        let material_filtered =
            run::MissingWmoPolicy::DeclareVacatedAfterRootAbsentM2OrEmptyOrUnselectedOrMaterialFiltered;
        assert_eq!(
            completion_profile_with_omissions(
                Target::Build70450,
                plan::MapSelection::All,
                run::MissingWdtPolicy::DeclareRootAbsent,
                run::MissingModelPolicy::DeclareRootAbsentWmoDoodadM2,
                material_filtered,
                Profile::SourceExpandedV1,
            ),
            Ok(PROFILE_70450_AVAILABLE_SOURCE_EXPANDED_MATERIAL_FILTERED_WMO_GEOMETRY)
        );
        for (target, selection, missing_wdt, missing_model, missing_wmo) in [
            (
                Target::Build70425,
                plan::MapSelection::All,
                run::MissingWdtPolicy::DeclareRootAbsent,
                run::MissingModelPolicy::DeclareRootAbsentWmoDoodadM2,
                material_filtered,
            ),
            (
                Target::Build70450,
                plan::MapSelection::MapAndAncestors(7),
                run::MissingWdtPolicy::DeclareRootAbsent,
                run::MissingModelPolicy::DeclareRootAbsentWmoDoodadM2,
                material_filtered,
            ),
            (
                Target::Build70450,
                plan::MapSelection::All,
                run::MissingWdtPolicy::Reject,
                run::MissingModelPolicy::DeclareRootAbsentWmoDoodadM2,
                material_filtered,
            ),
            (
                Target::Build70450,
                plan::MapSelection::All,
                run::MissingWdtPolicy::DeclareRootAbsent,
                run::MissingModelPolicy::Reject,
                material_filtered,
            ),
            (
                Target::Build70450,
                plan::MapSelection::All,
                run::MissingWdtPolicy::DeclareRootAbsent,
                run::MissingModelPolicy::DeclareRootAbsentWmoDoodadM2,
                run::MissingWmoPolicy::Reject,
            ),
        ] {
            assert_eq!(
                completion_profile_with_omissions(
                    target,
                    selection,
                    missing_wdt,
                    missing_model,
                    missing_wmo,
                    Profile::SourceExpandedV1,
                ),
                Err(Error::State)
            );
        }
    }
}
