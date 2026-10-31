// SPDX-License-Identifier: GPL-3.0-or-later
// Authored 2026-10-11: shared startup-to-WDT/ADT collision execution.
//! Explicit addition to the existing per-batch runner. One collector retains
//! startup RAW, dependency handles, omissions and all cumulative budgets. Only
//! the current Objects request window is projected. One run is admitted per
//! collector, including across lookup transitions; every failure is sticky.
//!
//! WDTs all read before globals, then serial depth/physical-map and x/y order.
//! These are declared scheduling rules, not native parallel-order equivalence.
//! Existing callers reject missing dependencies. Only the explicit WDT and
//! WMO-doodad policies tolerate guarded raw ROOT absences; the separate v34
//! policy also declares intrinsically empty WMO models, while v35 additionally
//! declares valid source geometry explicitly skipped before selection. Declarations are returned
//! in bounded, disjoint lists. Sink callbacks only stage. Startup models and each
//! subsequently inserted model are delivered once, including omission states.
//! No publication occurs.
//!
//! Run-owned source capacities, parser allowances, WDT/cache descriptors and
//! cache growth overlap share the collector's logical memory ceiling. A bridge
//! reservation covers its entire Session (including ID ceiling) and one Output.
//! Caller Maps/strings, Sink storage, stack and allocator overhead are excluded.
//! Existing cumulative collector ceilings may reject a complete large map; no
//! counter reset, truncated plan, implicit cache eviction or full-build claim.
use super::*;
use crate::collision::{Facts, gameobjects};

#[cfg(test)]
mod tests;

/// Only reductions of run component ceilings. The already established
/// collector owns all model, source-total, read, request and cache-entry limits.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SharedLimits {
    pub max_maps: usize,
    pub max_tile_positions: u64,
    /// WDT/ADT per-file cap; model reads use the collector's established cap.
    /// All source reads additionally obey that collector cap.
    pub max_file_bytes: usize,
    pub max_wdt_memory_bytes: usize,
    pub max_cache_memory_bytes: usize,
    pub max_output_bytes: u64,
    pub wdt: plan::Limits,
    pub objects: objects::Limits,
    pub placements: placements::Limits,
    pub cache: cache::Limits,
}
impl Default for SharedLimits {
    fn default() -> Self {
        let defaults = Limits::default();
        Self {
            max_maps: defaults.max_maps,
            max_tile_positions: defaults.max_tile_positions,
            max_file_bytes: defaults.max_file_bytes,
            max_wdt_memory_bytes: defaults.max_wdt_memory_bytes,
            max_cache_memory_bytes: defaults.max_cache_memory_bytes,
            max_output_bytes: defaults.max_output_bytes,
            wdt: defaults.wdt,
            objects: defaults.objects,
            placements: defaults.placements,
            cache: defaults.cache,
        }
    }
}
impl SharedLimits {
    #[cfg(target_os = "linux")]
    pub(crate) fn validate_resources(
        self,
        profile: super::super::resources::Profile,
    ) -> Result<(), Error> {
        let mut components = self.components();
        if profile.model_heavy() {
            components.placements.max_models =
                components.placements.max_models.min(placements::MAX_MODELS);
        }
        components.validate()?;
        placements::Session::new_with_resources(self.placements, profile)
            .map_err(Error::Placements)?;
        Ok(())
    }
    // Component-only use of existing validators/output accounting. The dummy
    // collision limits are never installed on or used to construct a collector.
    fn components(self) -> Limits {
        Limits {
            max_maps: self.max_maps,
            max_tile_positions: self.max_tile_positions,
            max_file_bytes: self.max_file_bytes,
            max_wdt_memory_bytes: self.max_wdt_memory_bytes,
            max_cache_memory_bytes: self.max_cache_memory_bytes,
            max_output_bytes: self.max_output_bytes,
            wdt: self.wdt,
            objects: self.objects,
            collision: super::super::Limits::default(),
            placements: self.placements,
            cache: self.cache,
        }
    }
    pub(crate) fn validate(self) -> Result<(), Error> {
        self.components().validate()?;
        placements::Session::new(self.placements).map_err(Error::Placements)?;
        Ok(())
    }
    #[cfg(target_os = "linux")]
    pub(crate) fn validate_resources_for(
        self,
        profile: super::super::resources::Profile,
    ) -> Result<(), Error> {
        self.validate_resources(profile)
    }
}
/// `run.model_reads/model_source_bytes` count only deltas during model requests,
/// excluding startup and WDT/ADT reads. Model callback/raw totals include the
/// initial inventory once. Collector snapshots cover all phases and sources.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SharedReport {
    pub run: Report,
    pub collector_before: Facts,
    pub collector_after: Facts,
    pub request_calls_before: usize,
    pub request_calls_after: usize,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MissingWdtPolicy {
    Reject,
    DeclareRootAbsent,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MissingModelPolicy {
    Reject,
    DeclareRootAbsentWmoDoodadM2,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MissingWmoPolicy {
    Reject,
    DeclareVacatedAfterRootAbsentM2,
    DeclareVacatedAfterRootAbsentM2OrEmpty,
    DeclareVacatedAfterRootAbsentM2OrEmptyOrUnselected,
    DeclareVacatedAfterRootAbsentM2OrEmptyOrUnselectedOrMaterialFiltered,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RootAbsent {
    pub map_id: u32,
    pub wdt_file_data_id: u32,
}

/// Successful values are ordered as `(report, root_absent, missing_m2, missing_wmo, empty_wmo)`.
pub type VacatedWmoResult =
    Result<(SharedReport, Vec<RootAbsent>, Vec<u32>, Vec<u32>, Vec<u32>), Error>;

/// Successful values are ordered as `(report, root_absent, missing_m2, missing_wmo, empty_wmo, unselected_wmo)`.
pub type UnselectedWmoResult = Result<
    (
        SharedReport,
        Vec<RootAbsent>,
        Vec<u32>,
        Vec<u32>,
        Vec<u32>,
        Vec<u32>,
    ),
    Error,
>;

/// Successful values are ordered as `(report, root_absent, missing_m2, missing_wmo, empty_wmo, unselected_wmo, material_filtered_wmo)`.
pub type MaterialFilteredWmoResult = Result<
    (
        SharedReport,
        Vec<RootAbsent>,
        Vec<u32>,
        Vec<u32>,
        Vec<u32>,
        Vec<u32>,
        Vec<u32>,
    ),
    Error,
>;

type SharedRunResult = Result<
    (
        SharedReport,
        Vec<RootAbsent>,
        Vec<u32>,
        Vec<u32>,
        Vec<u32>,
        Vec<u32>,
        Vec<u32>,
    ),
    Error,
>;

pub fn extract_shared<L: LiquidLookup + ?Sized, S: Sink + ?Sized>(
    context: &mut gameobjects::Context<'_, '_, L>,
    maps: &plan::Maps<'_>,
    sink: &mut S,
    limits: SharedLimits,
) -> Result<SharedReport, Error> {
    extract_collector(context.collector_mut(), maps, sink, limits)
}

pub fn extract_shared_with_policy<L: LiquidLookup + ?Sized, S: Sink + ?Sized>(
    context: &mut gameobjects::Context<'_, '_, L>,
    maps: &plan::Maps<'_>,
    sink: &mut S,
    limits: SharedLimits,
    missing_wdt: MissingWdtPolicy,
) -> Result<(SharedReport, Vec<RootAbsent>), Error> {
    extract_collector_with_policy(context.collector_mut(), maps, sink, limits, missing_wdt)
}

pub fn extract_shared_with_omissions<L: LiquidLookup + ?Sized, S: Sink + ?Sized>(
    context: &mut gameobjects::Context<'_, '_, L>,
    maps: &plan::Maps<'_>,
    sink: &mut S,
    limits: SharedLimits,
    missing_wdt: MissingWdtPolicy,
    missing_model: MissingModelPolicy,
) -> Result<(SharedReport, Vec<RootAbsent>, Vec<u32>), Error> {
    extract_collector_with_omissions(
        context.collector_mut(),
        maps,
        sink,
        limits,
        missing_wdt,
        missing_model,
    )
}

pub fn extract_shared_with_vacated_wmo<L: LiquidLookup + ?Sized, S: Sink + ?Sized>(
    context: &mut gameobjects::Context<'_, '_, L>,
    maps: &plan::Maps<'_>,
    sink: &mut S,
    limits: SharedLimits,
) -> VacatedWmoResult {
    extract_collector_with_vacated_wmo(
        context.collector_mut(),
        maps,
        sink,
        limits,
        MissingWdtPolicy::DeclareRootAbsent,
        MissingModelPolicy::DeclareRootAbsentWmoDoodadM2,
    )
}

pub(in crate::collision) fn extract_collector<L: LiquidLookup + ?Sized, S: Sink + ?Sized>(
    collector: &mut Extractor<'_, '_, L>,
    maps: &plan::Maps<'_>,
    sink: &mut S,
    limits: SharedLimits,
) -> Result<SharedReport, Error> {
    extract_collector_with_policy(collector, maps, sink, limits, MissingWdtPolicy::Reject)
        .map(|(report, _)| report)
}

pub(in crate::collision) fn extract_collector_with_policy<
    L: LiquidLookup + ?Sized,
    S: Sink + ?Sized,
>(
    collector: &mut Extractor<'_, '_, L>,
    maps: &plan::Maps<'_>,
    sink: &mut S,
    limits: SharedLimits,
    missing_wdt: MissingWdtPolicy,
) -> Result<(SharedReport, Vec<RootAbsent>), Error> {
    extract_collector_with_omissions(
        collector,
        maps,
        sink,
        limits,
        missing_wdt,
        MissingModelPolicy::Reject,
    )
    .map(|(report, root_absent, _)| (report, root_absent))
}

pub(in crate::collision) fn extract_collector_with_omissions<
    L: LiquidLookup + ?Sized,
    S: Sink + ?Sized,
>(
    collector: &mut Extractor<'_, '_, L>,
    maps: &plan::Maps<'_>,
    sink: &mut S,
    limits: SharedLimits,
    missing_wdt: MissingWdtPolicy,
    missing_model: MissingModelPolicy,
) -> Result<(SharedReport, Vec<RootAbsent>, Vec<u32>), Error> {
    let result = execute(
        collector,
        maps,
        sink,
        limits,
        missing_wdt,
        missing_model,
        MissingWmoPolicy::Reject,
    )
    .map(|(report, root_absent, missing_m2, _, _, _, _)| (report, root_absent, missing_m2));
    if result.is_err() {
        collector.abort();
    }
    result
}

pub(in crate::collision) fn extract_collector_with_vacated_wmo<
    L: LiquidLookup + ?Sized,
    S: Sink + ?Sized,
>(
    collector: &mut Extractor<'_, '_, L>,
    maps: &plan::Maps<'_>,
    sink: &mut S,
    limits: SharedLimits,
    missing_wdt: MissingWdtPolicy,
    missing_model: MissingModelPolicy,
) -> VacatedWmoResult {
    let result = execute(
        collector,
        maps,
        sink,
        limits,
        missing_wdt,
        missing_model,
        MissingWmoPolicy::DeclareVacatedAfterRootAbsentM2,
    )
    .map(
        |(report, root_absent, missing_m2, missing_wmo, empty_wmo, _, _)| {
            (report, root_absent, missing_m2, missing_wmo, empty_wmo)
        },
    );
    if result.is_err() {
        collector.abort();
    }
    result
}

#[cfg(target_os = "linux")]
pub(in crate::collision) fn extract_collector_with_empty_wmo<
    L: LiquidLookup + ?Sized,
    S: Sink + ?Sized,
>(
    collector: &mut Extractor<'_, '_, L>,
    maps: &plan::Maps<'_>,
    sink: &mut S,
    limits: SharedLimits,
    missing_wdt: MissingWdtPolicy,
    missing_model: MissingModelPolicy,
) -> VacatedWmoResult {
    let result = execute(
        collector,
        maps,
        sink,
        limits,
        missing_wdt,
        missing_model,
        MissingWmoPolicy::DeclareVacatedAfterRootAbsentM2OrEmpty,
    )
    .map(
        |(report, root_absent, missing_m2, missing_wmo, empty_wmo, _, _)| {
            (report, root_absent, missing_m2, missing_wmo, empty_wmo)
        },
    );
    if result.is_err() {
        collector.abort();
    }
    result
}

#[cfg(target_os = "linux")]
pub(in crate::collision) fn extract_collector_with_unselected_wmo<
    L: LiquidLookup + ?Sized,
    S: Sink + ?Sized,
>(
    collector: &mut Extractor<'_, '_, L>,
    maps: &plan::Maps<'_>,
    sink: &mut S,
    limits: SharedLimits,
    missing_wdt: MissingWdtPolicy,
    missing_model: MissingModelPolicy,
) -> UnselectedWmoResult {
    let result = execute(
        collector,
        maps,
        sink,
        limits,
        missing_wdt,
        missing_model,
        MissingWmoPolicy::DeclareVacatedAfterRootAbsentM2OrEmptyOrUnselected,
    )
    .map(
        |(report, root_absent, missing_m2, missing_wmo, empty_wmo, unselected_wmo, _)| {
            (
                report,
                root_absent,
                missing_m2,
                missing_wmo,
                empty_wmo,
                unselected_wmo,
            )
        },
    );
    if result.is_err() {
        collector.abort();
    }
    result
}

#[cfg(target_os = "linux")]
pub fn extract_shared_with_unselected_wmo<L: LiquidLookup + ?Sized, S: Sink + ?Sized>(
    context: &mut gameobjects::Context<'_, '_, L>,
    maps: &plan::Maps<'_>,
    sink: &mut S,
    limits: SharedLimits,
) -> UnselectedWmoResult {
    extract_collector_with_unselected_wmo(
        context.collector_mut(),
        maps,
        sink,
        limits,
        MissingWdtPolicy::DeclareRootAbsent,
        MissingModelPolicy::DeclareRootAbsentWmoDoodadM2,
    )
}

#[cfg(target_os = "linux")]
pub(in crate::collision) fn extract_collector_with_material_filtered_wmo<
    L: LiquidLookup + ?Sized,
    S: Sink + ?Sized,
>(
    collector: &mut Extractor<'_, '_, L>,
    maps: &plan::Maps<'_>,
    sink: &mut S,
    limits: SharedLimits,
    missing_wdt: MissingWdtPolicy,
    missing_model: MissingModelPolicy,
) -> MaterialFilteredWmoResult {
    let result = execute(
        collector,
        maps,
        sink,
        limits,
        missing_wdt,
        missing_model,
        MissingWmoPolicy::DeclareVacatedAfterRootAbsentM2OrEmptyOrUnselectedOrMaterialFiltered,
    );
    if result.is_err() {
        collector.abort();
    }
    result
}

#[cfg(target_os = "linux")]
pub fn extract_shared_with_material_filtered_wmo<L: LiquidLookup + ?Sized, S: Sink + ?Sized>(
    context: &mut gameobjects::Context<'_, '_, L>,
    maps: &plan::Maps<'_>,
    sink: &mut S,
    limits: SharedLimits,
) -> MaterialFilteredWmoResult {
    extract_collector_with_material_filtered_wmo(
        context.collector_mut(),
        maps,
        sink,
        limits,
        MissingWdtPolicy::DeclareRootAbsent,
        MissingModelPolicy::DeclareRootAbsentWmoDoodadM2,
    )
}

struct Run<'a, 's, 'l, L: LiquidLookup + ?Sized, S: Sink + ?Sized> {
    collector: &'a mut Extractor<'s, 'l, L>,
    sink: &'a mut S,
    session: placements::Session,
    limits: Limits,
    report: Report,
    emitted: usize,
    cache_budget: CacheBudget,
    missing_model: MissingModelPolicy,
    missing_wmo_policy: MissingWmoPolicy,
    missing_m2: Vec<u32>,
    missing_m2_limit: usize,
    missing_wmo: Vec<u32>,
    missing_wmo_limit: usize,
    empty_wmo: Vec<u32>,
    empty_wmo_limit: usize,
    unselected_wmo: Vec<u32>,
    unselected_wmo_limit: usize,
    material_filtered_wmo: Vec<u32>,
    material_filtered_wmo_limit: usize,
}
enum WdtState {
    Present(Vec<u8>),
    RootAbsent,
}
struct SharedMapState {
    id: u32,
    wdt: WdtState,
    cache: Vec<TileCache>,
}
fn execute<L: LiquidLookup + ?Sized, S: Sink + ?Sized>(
    collector: &mut Extractor<'_, '_, L>,
    maps: &plan::Maps<'_>,
    sink: &mut S,
    limits: SharedLimits,
    missing_wdt: MissingWdtPolicy,
    missing_model: MissingModelPolicy,
    missing_wmo_policy: MissingWmoPolicy,
) -> SharedRunResult {
    collector.begin_shared_run().map_err(Error::Collision)?;
    #[cfg(target_os = "linux")]
    limits.validate_resources_for(collector.resources())?;
    #[cfg(not(target_os = "linux"))]
    limits.validate()?;
    let before = collector.facts();
    let calls_before = collector.request_calls();
    let count = maps.preload().len();
    let positions = (count as u64).checked_mul(4096).ok_or(Error::Limit)?;
    if count > limits.max_maps || positions > limits.max_tile_positions {
        return Err(Error::Limit);
    }
    let bridge = limits
        .placements
        .max_memory_bytes
        .min(collector.free_bytes().map_err(Error::Collision)?);
    if bridge < limits.placements.ids.max_memory_bytes {
        return Err(Error::Limit);
    }
    collector
        .reserve_external(bridge)
        .map_err(Error::Collision)?;
    #[cfg(target_os = "linux")]
    let session = placements::Session::new_with_resources(
        placements::Limits {
            max_memory_bytes: bridge,
            ..limits.placements
        },
        collector.resources(),
    )
    .map_err(Error::Placements)?;
    #[cfg(not(target_os = "linux"))]
    let session = placements::Session::new(placements::Limits {
        max_memory_bytes: bridge,
        ..limits.placements
    })
    .map_err(Error::Placements)?;
    let requested = count
        .checked_mul(size_of::<SharedMapState>())
        .ok_or(Error::Limit)?;
    let declaration_requested = match missing_wdt {
        MissingWdtPolicy::Reject => 0,
        MissingWdtPolicy::DeclareRootAbsent => count
            .checked_mul(size_of::<RootAbsent>())
            .ok_or(Error::Limit)?,
    };
    if requested
        .checked_add(declaration_requested)
        .ok_or(Error::Limit)?
        > limits.max_wdt_memory_bytes
    {
        return Err(Error::Limit);
    }
    collector
        .reserve_external(requested)
        .map_err(Error::Collision)?;
    let mut states = Vec::new();
    states
        .try_reserve_exact(count)
        .map_err(|_| Error::Allocation)?;
    let descriptors = capacity(&states)?;
    if descriptors > limits.max_wdt_memory_bytes {
        return Err(Error::Limit);
    }
    collector
        .reserve_external(descriptors.checked_sub(requested).ok_or(Error::State)?)
        .map_err(Error::Collision)?;
    let mut root_absent = Vec::new();
    if declaration_requested != 0 {
        collector
            .reserve_external(declaration_requested)
            .map_err(Error::Collision)?;
        root_absent
            .try_reserve_exact(count)
            .map_err(|_| Error::Allocation)?;
    }
    let declaration_descriptors = capacity(&root_absent)?;
    if declaration_descriptors > declaration_requested {
        collector
            .reserve_external(declaration_descriptors - declaration_requested)
            .map_err(Error::Collision)?;
    }
    let descriptor_resident = descriptors
        .checked_add(declaration_descriptors)
        .ok_or(Error::Limit)?;
    if descriptor_resident > limits.max_wdt_memory_bytes {
        return Err(Error::Limit);
    }
    let mut wdt_resident = descriptor_resident;
    let mut wdt_files = 0_u64;
    #[cfg(target_os = "linux")]
    collector.set_product_diagnostic_phase(local_store::ProductDiagnosticPhase::Placements);
    for map in maps.preload() {
        let id = map.wdt_file_data_id();
        let cap = limits
            .max_file_bytes
            .min(limits.wdt.max_wdt_bytes)
            .min(limits.max_wdt_memory_bytes - wdt_resident);
        let (bytes, _) = match missing_wdt {
            MissingWdtPolicy::Reject => collector
                .read_source_ref_with_diagnostic(
                    InputRef::FileDataId(id),
                    cap,
                    local_store::ProductDiagnosticRequestKind::Wdt,
                )
                .map_err(Error::Collision)?,
            MissingWdtPolicy::DeclareRootAbsent => {
                match collector
                    .read_wdt_with_root_absence(id, cap)
                    .map_err(Error::Collision)?
                {
                    crate::collision::WdtRead::Present(bytes, facts) => (bytes, facts),
                    crate::collision::WdtRead::RootAbsent(absent_id) => {
                        if absent_id != id || id == 0 || root_absent.len() == count {
                            return Err(Error::State);
                        }
                        root_absent.push(RootAbsent {
                            map_id: map.id(),
                            wdt_file_data_id: id,
                        });
                        states.push(SharedMapState {
                            id: map.id(),
                            wdt: WdtState::RootAbsent,
                            cache: Vec::new(),
                        });
                        continue;
                    }
                }
            }
        };
        bump(&mut wdt_files, 1)?;
        wdt_resident = wdt_resident
            .checked_add(bytes.capacity())
            .filter(|n| *n <= limits.max_wdt_memory_bytes)
            .ok_or(Error::Limit)?;
        states.push(SharedMapState {
            id: map.id(),
            wdt: WdtState::Present(bytes),
            cache: Vec::new(),
        });
    }
    let omission_limit = limits
        .placements
        .max_models
        .min(collector.model_limit())
        .min(placements::MAX_MODELS);
    let missing_m2_limit = match missing_model {
        MissingModelPolicy::Reject => 0,
        MissingModelPolicy::DeclareRootAbsentWmoDoodadM2 => omission_limit,
    };
    let missing_m2_requested = missing_m2_limit
        .checked_mul(size_of::<u32>())
        .ok_or(Error::Limit)?;
    let mut missing_m2 = Vec::new();
    if missing_m2_requested != 0 {
        collector
            .reserve_external(missing_m2_requested)
            .map_err(Error::Collision)?;
        missing_m2
            .try_reserve_exact(missing_m2_limit)
            .map_err(|_| Error::Allocation)?;
    }
    let missing_m2_descriptors = capacity(&missing_m2)?;
    if missing_m2_descriptors > missing_m2_requested {
        collector
            .reserve_external(missing_m2_descriptors - missing_m2_requested)
            .map_err(Error::Collision)?;
    }
    let wmo_declaration_limit = match missing_wmo_policy {
        MissingWmoPolicy::Reject => 0,
        MissingWmoPolicy::DeclareVacatedAfterRootAbsentM2
        | MissingWmoPolicy::DeclareVacatedAfterRootAbsentM2OrEmpty
        | MissingWmoPolicy::DeclareVacatedAfterRootAbsentM2OrEmptyOrUnselected
        | MissingWmoPolicy::DeclareVacatedAfterRootAbsentM2OrEmptyOrUnselectedOrMaterialFiltered => {
            omission_limit
        }
    };
    let empty_wmo_limit = match missing_wmo_policy {
        MissingWmoPolicy::DeclareVacatedAfterRootAbsentM2OrEmpty
        | MissingWmoPolicy::DeclareVacatedAfterRootAbsentM2OrEmptyOrUnselected
        | MissingWmoPolicy::DeclareVacatedAfterRootAbsentM2OrEmptyOrUnselectedOrMaterialFiltered => {
            wmo_declaration_limit
        }
        MissingWmoPolicy::Reject | MissingWmoPolicy::DeclareVacatedAfterRootAbsentM2 => 0,
    };
    let unselected_wmo_limit = match missing_wmo_policy {
        MissingWmoPolicy::DeclareVacatedAfterRootAbsentM2OrEmptyOrUnselected
        | MissingWmoPolicy::DeclareVacatedAfterRootAbsentM2OrEmptyOrUnselectedOrMaterialFiltered => {
            wmo_declaration_limit
        }
        MissingWmoPolicy::Reject
        | MissingWmoPolicy::DeclareVacatedAfterRootAbsentM2
        | MissingWmoPolicy::DeclareVacatedAfterRootAbsentM2OrEmpty => 0,
    };
    let missing_wmo_requested = wmo_declaration_limit
        .checked_mul(size_of::<u32>())
        .ok_or(Error::Limit)?;
    let empty_wmo_requested = empty_wmo_limit
        .checked_mul(size_of::<u32>())
        .ok_or(Error::Limit)?;
    let unselected_wmo_requested = unselected_wmo_limit
        .checked_mul(size_of::<u32>())
        .ok_or(Error::Limit)?;
    let material_filtered_wmo_limit = match missing_wmo_policy {
        MissingWmoPolicy::DeclareVacatedAfterRootAbsentM2OrEmptyOrUnselectedOrMaterialFiltered => {
            wmo_declaration_limit
        }
        MissingWmoPolicy::Reject
        | MissingWmoPolicy::DeclareVacatedAfterRootAbsentM2
        | MissingWmoPolicy::DeclareVacatedAfterRootAbsentM2OrEmpty
        | MissingWmoPolicy::DeclareVacatedAfterRootAbsentM2OrEmptyOrUnselected => 0,
    };
    let material_filtered_wmo_requested = material_filtered_wmo_limit
        .checked_mul(size_of::<u32>())
        .ok_or(Error::Limit)?;
    let additional_declarations_requested = missing_m2_descriptors
        .checked_add(missing_wmo_requested)
        .and_then(|resident| resident.checked_add(empty_wmo_requested))
        .and_then(|resident| resident.checked_add(unselected_wmo_requested))
        .and_then(|resident| resident.checked_add(material_filtered_wmo_requested))
        .ok_or(Error::Limit)?;
    wdt_resident
        .checked_add(additional_declarations_requested)
        .filter(|resident| *resident <= limits.max_wdt_memory_bytes)
        .ok_or(Error::Limit)?;
    let mut missing_wmo = Vec::new();
    let mut empty_wmo = Vec::new();
    let mut unselected_wmo = Vec::new();
    let mut material_filtered_wmo = Vec::new();
    if missing_wmo_requested != 0 {
        collector
            .reserve_external(missing_wmo_requested)
            .map_err(Error::Collision)?;
        missing_wmo
            .try_reserve_exact(wmo_declaration_limit)
            .map_err(|_| Error::Allocation)?;
    }
    if empty_wmo_requested != 0 {
        collector
            .reserve_external(empty_wmo_requested)
            .map_err(Error::Collision)?;
        empty_wmo
            .try_reserve_exact(empty_wmo_limit)
            .map_err(|_| Error::Allocation)?;
    }
    if unselected_wmo_requested != 0 {
        collector
            .reserve_external(unselected_wmo_requested)
            .map_err(Error::Collision)?;
        unselected_wmo
            .try_reserve_exact(unselected_wmo_limit)
            .map_err(|_| Error::Allocation)?;
    }
    if material_filtered_wmo_requested != 0 {
        collector
            .reserve_external(material_filtered_wmo_requested)
            .map_err(Error::Collision)?;
        material_filtered_wmo
            .try_reserve_exact(material_filtered_wmo_limit)
            .map_err(|_| Error::Allocation)?;
    }
    let missing_wmo_descriptors = capacity(&missing_wmo)?;
    let empty_wmo_descriptors = capacity(&empty_wmo)?;
    let unselected_wmo_descriptors = capacity(&unselected_wmo)?;
    let material_filtered_wmo_descriptors = capacity(&material_filtered_wmo)?;
    if missing_wmo_descriptors > missing_wmo_requested {
        collector
            .reserve_external(missing_wmo_descriptors - missing_wmo_requested)
            .map_err(Error::Collision)?;
    }
    if empty_wmo_descriptors > empty_wmo_requested {
        collector
            .reserve_external(empty_wmo_descriptors - empty_wmo_requested)
            .map_err(Error::Collision)?;
    }
    if unselected_wmo_descriptors > unselected_wmo_requested {
        collector
            .reserve_external(unselected_wmo_descriptors - unselected_wmo_requested)
            .map_err(Error::Collision)?;
    }
    if material_filtered_wmo_descriptors > material_filtered_wmo_requested {
        collector
            .reserve_external(material_filtered_wmo_descriptors - material_filtered_wmo_requested)
            .map_err(Error::Collision)?;
    }
    wdt_resident
        .checked_add(missing_m2_descriptors)
        .and_then(|resident| resident.checked_add(missing_wmo_descriptors))
        .and_then(|resident| resident.checked_add(empty_wmo_descriptors))
        .and_then(|resident| resident.checked_add(unselected_wmo_descriptors))
        .and_then(|resident| resident.checked_add(material_filtered_wmo_descriptors))
        .filter(|resident| *resident <= limits.max_wdt_memory_bytes)
        .ok_or(Error::Limit)?;
    let report = Report {
        build: before.build,
        maps: count as u64,
        wdt_files,
        object_adts: 0,
        tile_positions: positions,
        own_tiles: 0,
        replayed_tiles: 0,
        empty_replays: 0,
        cached_tiles: 0,
        model_callbacks: 0,
        collision_models: 0,
        no_collision_models: 0,
        doodad_only_models: 0,
        model_reads: 0,
        model_source_bytes: 0,
        model_raw_bytes: 0,
        placement_callbacks: 0,
        placement_records: 0,
        placement_bytes: 0,
        output_bytes: 0,
        wdt_resident_bytes: wdt_resident
            .checked_sub(declaration_descriptors)
            .ok_or(Error::State)?,
        cache_resident_bytes: 0,
        cache_peak_bytes: 0,
        ids: session.id_stats(),
    };
    let mut run = Run {
        collector,
        sink,
        session,
        limits: limits.components(),
        report,
        emitted: 0,
        cache_budget: CacheBudget {
            live: 0,
            peak: 0,
            cap: limits.max_cache_memory_bytes,
        },
        missing_model,
        missing_m2,
        missing_m2_limit,
        missing_wmo_policy,
        missing_wmo,
        missing_wmo_limit: wmo_declaration_limit,
        empty_wmo,
        empty_wmo_limit,
        unselected_wmo,
        unselected_wmo_limit,
        material_filtered_wmo,
        material_filtered_wmo_limit,
    };
    run.models()?;
    for (map, state) in maps.preload().zip(&states) {
        if state.id != map.id() {
            return Err(Error::State);
        }
        let bytes = match &state.wdt {
            WdtState::Present(bytes) => bytes,
            WdtState::RootAbsent => {
                if !state.cache.is_empty() {
                    return Err(Error::State);
                }
                continue;
            }
        };
        let wdt = run.wdt(bytes)?;
        wdt.tiles(map).map_err(Error::Plan)?;
        run.project(
            wdt.globals(),
            Context {
                map_id: map.id(),
                original_map_id: map.id(),
                kind: Kind::Global,
                replayed: false,
            },
        )?;
        let resident = wdt.stats().resident_bytes;
        drop(wdt);
        run.collector
            .release_external(resident)
            .map_err(Error::Collision)?;
    }
    for map in maps.processing() {
        let index = states
            .iter()
            .position(|state| state.id == map.id())
            .ok_or(Error::State)?;
        let (prior, rest) = states.split_at_mut(index);
        let (current, after) = rest.split_first_mut().ok_or(Error::State)?;
        if current.id != map.id() {
            return Err(Error::State);
        }
        let bytes = match &current.wdt {
            WdtState::Present(bytes) => bytes,
            WdtState::RootAbsent => {
                if !current.cache.is_empty() {
                    return Err(Error::State);
                }
                continue;
            }
        };
        let wdt = run.wdt(bytes)?;
        let mut tiles = wdt.tiles(map).map_err(Error::Plan)?;
        let parent = map.parent_map_id().and_then(|id| {
            prior
                .iter()
                .chain(after.iter())
                .find(|state| state.id == id)
        });
        let mut names = [0; plan::MAX_NAME_BYTES];
        let mut cursor = 0u16;
        while let Some(tile) = tiles.next(&mut names) {
            let cell = u16::from(tile.x) * 64 + u16::from(tile.y);
            while cursor < cell {
                run.replay(maps, map, parent, cursor)?;
                cursor += 1;
            }
            let target = match tile.target {
                plan::TileTarget::FileDataId(id) => InputRef::FileDataId(id),
                plan::TileTarget::Name(name) => InputRef::LogicalName(name),
            };
            #[cfg(target_os = "linux")]
            run.collector
                .set_product_diagnostic_phase(local_store::ProductDiagnosticPhase::Placements);
            let (bytes, _) = run
                .collector
                .read_source_ref_with_diagnostic(
                    target,
                    limits.max_file_bytes.min(limits.objects.max_input_bytes),
                    local_store::ProductDiagnosticRequestKind::TileAdt,
                )
                .map_err(Error::Collision)?;
            bump(&mut run.report.object_adts, 1)?;
            let objects = run.objects(&bytes)?;
            let output = run.project(
                &objects,
                Context {
                    map_id: map.id(),
                    original_map_id: map.id(),
                    kind: Kind::Tile {
                        x: tile.x,
                        y: tile.y,
                    },
                    replayed: false,
                },
            )?;
            bump(&mut run.report.own_tiles, 1)?;
            if map.is_parent() {
                let batch = output.batches().first().ok_or(Error::State)?;
                run.cache(&mut current.cache, cell, batch.bytes())?;
                bump(&mut run.report.cached_tiles, 1)?;
            }
            drop(output); // still covered by the lifetime bridge reservation
            let object_resident = objects.stats().resident_bytes;
            drop(objects);
            run.collector
                .release_external(object_resident)
                .map_err(Error::Collision)?;
            let source_capacity = bytes.capacity();
            drop(bytes);
            run.collector
                .release_source(source_capacity)
                .map_err(Error::Collision)?;
            cursor = cell + 1;
        }
        while cursor < 4096 {
            run.replay(maps, map, parent, cursor)?;
            cursor += 1;
        }
        let resident = wdt.stats().resident_bytes;
        drop(wdt);
        run.collector
            .release_external(resident)
            .map_err(Error::Collision)?;
    }
    run.collector.verify().map_err(Error::Collision)?;
    run.report.cache_resident_bytes = run.cache_budget.live;
    run.report.cache_peak_bytes = run.cache_budget.peak;
    run.report.ids = run.session.id_stats();
    let declared = u64::try_from(root_absent.len()).map_err(|_| Error::Limit)?;
    if run.report.maps != count as u64
        || run
            .report
            .maps
            .checked_sub(run.report.wdt_files)
            .ok_or(Error::State)?
            != declared
    {
        return Err(Error::State);
    }
    drop(states);
    run.collector
        .release_source(
            wdt_resident
                .checked_sub(descriptors)
                .and_then(|bytes| bytes.checked_sub(declaration_descriptors))
                .ok_or(Error::State)?,
        )
        .map_err(Error::Collision)?;
    run.collector
        .release_external(
            descriptors
                .checked_add(run.cache_budget.live)
                .ok_or(Error::Limit)?,
        )
        .map_err(Error::Collision)?;
    if declaration_descriptors != 0 {
        run.collector
            .release_external(declaration_descriptors)
            .map_err(Error::Collision)?;
    }
    if missing_m2_descriptors != 0 {
        run.collector
            .release_external(missing_m2_descriptors)
            .map_err(Error::Collision)?;
    }
    if missing_wmo_descriptors != 0 {
        run.collector
            .release_external(missing_wmo_descriptors)
            .map_err(Error::Collision)?;
    }
    if empty_wmo_descriptors != 0 {
        run.collector
            .release_external(empty_wmo_descriptors)
            .map_err(Error::Collision)?;
    }
    if unselected_wmo_descriptors != 0 {
        run.collector
            .release_external(unselected_wmo_descriptors)
            .map_err(Error::Collision)?;
    }
    if material_filtered_wmo_descriptors != 0 {
        run.collector
            .release_external(material_filtered_wmo_descriptors)
            .map_err(Error::Collision)?;
    }
    run.missing_m2.sort_unstable();
    run.missing_m2.dedup();
    if run.missing_m2.len()
        > run
            .limits
            .placements
            .max_models
            .min(run.collector.model_limit())
        || run.missing_m2.windows(2).any(|ids| ids[0] >= ids[1])
    {
        return Err(Error::State);
    }
    run.missing_wmo.sort_unstable();
    run.missing_wmo.dedup();
    if run.missing_wmo.len() > run.missing_wmo_limit
        || run.missing_wmo.windows(2).any(|ids| ids[0] >= ids[1])
        || (matches!(
            run.missing_wmo_policy,
            MissingWmoPolicy::DeclareVacatedAfterRootAbsentM2OrEmptyOrUnselected
                | MissingWmoPolicy::DeclareVacatedAfterRootAbsentM2OrEmptyOrUnselectedOrMaterialFiltered
        ) && run
            .missing_m2
            .iter()
            .any(|id| run.missing_wmo.binary_search(id).is_ok()))
    {
        return Err(Error::State);
    }
    run.empty_wmo.sort_unstable();
    run.empty_wmo.dedup();
    if run.empty_wmo.len() > run.empty_wmo_limit
        || run.empty_wmo.windows(2).any(|ids| ids[0] >= ids[1])
        || run
            .missing_wmo
            .iter()
            .any(|id| run.empty_wmo.binary_search(id).is_ok())
        || (matches!(
            run.missing_wmo_policy,
            MissingWmoPolicy::DeclareVacatedAfterRootAbsentM2OrEmptyOrUnselected
                | MissingWmoPolicy::DeclareVacatedAfterRootAbsentM2OrEmptyOrUnselectedOrMaterialFiltered
        ) && run
            .missing_m2
            .iter()
            .any(|id| run.empty_wmo.binary_search(id).is_ok()))
    {
        return Err(Error::State);
    }
    run.unselected_wmo.sort_unstable();
    run.unselected_wmo.dedup();
    if run.unselected_wmo.len() > run.unselected_wmo_limit
        || run.unselected_wmo.windows(2).any(|ids| ids[0] >= ids[1])
        || run.unselected_wmo.iter().any(|id| {
            run.missing_m2.binary_search(id).is_ok()
                || run.missing_wmo.binary_search(id).is_ok()
                || run.empty_wmo.binary_search(id).is_ok()
        })
        || (run.missing_wmo_policy
            == MissingWmoPolicy::DeclareVacatedAfterRootAbsentM2OrEmptyOrUnselectedOrMaterialFiltered
            && (run.missing_m2.iter().any(|id| run.unselected_wmo.binary_search(id).is_ok())
                || run
                    .missing_wmo
                    .iter()
                    .any(|id| run.unselected_wmo.binary_search(id).is_ok())
                || run
                    .empty_wmo
                    .iter()
                    .any(|id| run.unselected_wmo.binary_search(id).is_ok())))
    {
        return Err(Error::State);
    }
    run.material_filtered_wmo.sort_unstable();
    run.material_filtered_wmo.dedup();
    if run.material_filtered_wmo.len() > run.material_filtered_wmo_limit
        || run
            .material_filtered_wmo
            .windows(2)
            .any(|ids| ids[0] >= ids[1])
        || run.material_filtered_wmo.iter().any(|id| {
            run.missing_m2.binary_search(id).is_ok()
                || run.missing_wmo.binary_search(id).is_ok()
                || run.empty_wmo.binary_search(id).is_ok()
                || run.unselected_wmo.binary_search(id).is_ok()
        })
        || (run.missing_wmo_policy
            == MissingWmoPolicy::DeclareVacatedAfterRootAbsentM2OrEmptyOrUnselectedOrMaterialFiltered
            && (run
                .missing_m2
                .iter()
                .any(|id| run.material_filtered_wmo.binary_search(id).is_ok())
                || run
                    .missing_wmo
                    .iter()
                    .any(|id| run.material_filtered_wmo.binary_search(id).is_ok())
                || run
                    .empty_wmo
                    .iter()
                    .any(|id| run.material_filtered_wmo.binary_search(id).is_ok())
                || run
                    .unselected_wmo
                    .iter()
                    .any(|id| run.material_filtered_wmo.binary_search(id).is_ok())))
    {
        return Err(Error::State);
    }
    drop(run.session);
    run.collector
        .release_external(bridge)
        .map_err(Error::Collision)?;
    Ok((
        SharedReport {
            run: run.report,
            collector_before: before,
            collector_after: run.collector.facts(),
            request_calls_before: calls_before,
            request_calls_after: run.collector.request_calls(),
        },
        root_absent,
        std::mem::take(&mut run.missing_m2),
        std::mem::take(&mut run.missing_wmo),
        std::mem::take(&mut run.empty_wmo),
        std::mem::take(&mut run.unselected_wmo),
        std::mem::take(&mut run.material_filtered_wmo),
    ))
}
impl<L: LiquidLookup + ?Sized, S: Sink + ?Sized> Run<'_, '_, '_, L, S> {
    fn wdt<'b>(&mut self, bytes: &'b [u8]) -> Result<plan::Wdt<'b>, Error> {
        let allowance = self
            .limits
            .wdt
            .max_memory_bytes
            .min(self.collector.free_bytes().map_err(Error::Collision)?);
        self.collector
            .reserve_external(allowance)
            .map_err(Error::Collision)?;
        let value = plan::Wdt::parse(
            bytes,
            plan::Limits {
                max_memory_bytes: allowance,
                ..self.limits.wdt
            },
        )
        .map_err(Error::Plan)?;
        let resident = value.stats().resident_bytes;
        self.collector
            .release_external(allowance.checked_sub(resident).ok_or(Error::State)?)
            .map_err(Error::Collision)?;
        Ok(value)
    }
    fn objects<'b>(&mut self, bytes: &'b [u8]) -> Result<Objects<'b>, Error> {
        let allowance = self
            .limits
            .objects
            .max_memory_bytes
            .min(self.collector.free_bytes().map_err(Error::Collision)?);
        self.collector
            .reserve_external(allowance)
            .map_err(Error::Collision)?;
        let value = Objects::parse_adt(
            bytes,
            objects::Limits {
                max_memory_bytes: allowance,
                ..self.limits.objects
            },
        )
        .map_err(Error::Objects)?;
        self.collector
            .release_external(
                allowance
                    .checked_sub(value.stats().resident_bytes)
                    .ok_or(Error::State)?,
            )
            .map_err(Error::Collision)?;
        Ok(value)
    }
    fn models(&mut self) -> Result<(), Error> {
        for model in &self.collector.models()[self.emitted..] {
            let bytes = model.raw().map_or(0, <[u8]>::len) as u64;
            output_bytes(&mut self.report, bytes, self.limits)?;
            self.sink.model(model).map_err(|_| Error::Sink)?;
            bump(&mut self.report.model_callbacks, 1)?;
            bump(&mut self.report.model_raw_bytes, bytes)?;
            bump(
                match model.status() {
                    Status::Collision => &mut self.report.collision_models,
                    Status::NoCollision | Status::Vacated => &mut self.report.no_collision_models,
                    Status::DoodadOnly => &mut self.report.doodad_only_models,
                },
                1,
            )?;
        }
        self.emitted = self.collector.models().len();
        Ok(())
    }
    fn project(
        &mut self,
        objects: &Objects<'_>,
        context: Context,
    ) -> Result<placements::Output, Error> {
        #[cfg(target_os = "linux")]
        self.collector
            .set_product_diagnostic_phase(local_store::ProductDiagnosticPhase::Models);
        let start = self.collector.request_len();
        let before = self.collector.facts();
        for request in objects.requests() {
            let target = match request.target() {
                Target::FileDataId(id) => InputRef::FileDataId(id),
                Target::Name(name) => InputRef::LogicalName(name),
            };
            let request = match request.source() {
                Source::M2(_) => ModelRequest::M2(target),
                Source::Wmo(_) => ModelRequest::Wmo(target),
            };
            if self.missing_model == MissingModelPolicy::Reject {
                self.collector
                    .extract_placement(request)
                    .map_err(Error::Collision)?;
            } else {
                let mut omissions = OmissionOptions {
                    missing_model: self.missing_model,
                    missing_wmo: self.missing_wmo_policy,
                    missing_m2: &mut self.missing_m2,
                    missing_m2_limit: self.missing_m2_limit,
                    missing_wmo_ids: &mut self.missing_wmo,
                    missing_wmo_limit: self.missing_wmo_limit,
                    empty_wmo_ids: &mut self.empty_wmo,
                    empty_wmo_limit: self.empty_wmo_limit,
                    unselected_wmo_ids: &mut self.unselected_wmo,
                    unselected_wmo_limit: self.unselected_wmo_limit,
                    material_filtered_wmo_ids: &mut self.material_filtered_wmo,
                    material_filtered_wmo_limit: self.material_filtered_wmo_limit,
                };
                self.collector
                    .extract_placement_with_omissions(request, &mut omissions)
                    .map_err(Error::Collision)?;
            }
        }
        let after = self.collector.facts();
        bump(
            &mut self.report.model_reads,
            after.reads.checked_sub(before.reads).ok_or(Error::State)? as u64,
        )?;
        bump(
            &mut self.report.model_source_bytes,
            after
                .source_bytes
                .checked_sub(before.source_bytes)
                .ok_or(Error::State)? as u64,
        )?;
        self.models()?;
        let view = self
            .collector
            .request_view(start, self.collector.request_len())
            .map_err(Error::Collision)?;
        let batches = [placements::Batch {
            objects,
            map_id: context.map_id,
            original_map_id: context.original_map_id,
        }];
        let output = if self.missing_model == MissingModelPolicy::Reject
            && self.missing_wmo_policy == MissingWmoPolicy::Reject
        {
            self.session.project_view(view, &batches)
        } else {
            self.session
                .project_view_with_omissions(view, &batches, &self.missing_m2)
        }
        .map_err(Error::Placements)?;
        let batch = output.batches().first().ok_or(Error::State)?;
        emit(
            self.sink,
            context,
            batch.bytes(),
            batch.record_count(),
            self.limits,
            &mut self.report,
        )?;
        Ok(output)
    }
    fn cache(
        &mut self,
        entries: &mut Vec<TileCache>,
        cell: u16,
        bytes: &[u8],
    ) -> Result<(), Error> {
        let allowance = self
            .limits
            .cache
            .max_memory_bytes
            .min(self.cache_budget.cap - self.cache_budget.live)
            .min(self.collector.free_bytes().map_err(Error::Collision)?);
        self.collector
            .reserve_external(allowance)
            .map_err(Error::Collision)?;
        let value = cache::Cached::from_bytes(
            bytes,
            cache::Limits {
                max_memory_bytes: allowance,
                ..self.limits.cache
            },
        )
        .map_err(Error::Placement)?;
        self.collector
            .release_external(
                allowance
                    .checked_sub(value.resident_bytes())
                    .ok_or(Error::State)?,
            )
            .map_err(Error::Collision)?;
        self.cache_budget.charge(value.resident_bytes())?;
        if entries.len() >= 4096 {
            return Err(Error::Limit);
        }
        if entries.len() == entries.capacity() {
            let old = capacity(entries)?;
            let next = entries.capacity().saturating_mul(2).clamp(8, 4096);
            let requested = next
                .checked_mul(size_of::<TileCache>())
                .ok_or(Error::Limit)?;
            self.cache_budget.check(requested)?;
            self.collector
                .reserve_external(requested)
                .map_err(Error::Collision)?;
            entries
                .try_reserve_exact(next - entries.len())
                .map_err(|_| Error::Allocation)?;
            let actual = capacity(entries)?;
            self.cache_budget.peak = self.cache_budget.peak.max(self.cache_budget.check(actual)?);
            self.collector
                .reserve_external(actual.checked_sub(requested).ok_or(Error::State)?)
                .map_err(Error::Collision)?;
            self.collector
                .release_external(old)
                .map_err(Error::Collision)?;
            self.cache_budget.live = self.cache_budget.live - old + actual;
        }
        entries.push(TileCache { cell, value });
        Ok(())
    }
    fn replay(
        &mut self,
        maps: &plan::Maps<'_>,
        map: plan::MapEntry<'_>,
        parent: Option<&SharedMapState>,
        cell: u16,
    ) -> Result<(), Error> {
        let Some(parent_map) = maps.fallback_parent(map.id()).map_err(Error::Plan)? else {
            return Ok(());
        };
        let parent = parent
            .filter(|state| state.id == parent_map.id())
            .ok_or(Error::State)?;
        match &parent.wdt {
            WdtState::RootAbsent if !parent.cache.is_empty() => return Err(Error::State),
            WdtState::RootAbsent => return Ok(()),
            WdtState::Present(_) => {}
        }
        let Ok(at) = parent.cache.binary_search_by_key(&cell, |entry| entry.cell) else {
            return Ok(());
        };
        let cached = &parent.cache[at].value;
        let old = cached.resident_bytes();
        let allowance = self
            .limits
            .cache
            .max_memory_bytes
            .checked_sub(old)
            .ok_or(Error::Limit)?
            .min(self.collector.free_bytes().map_err(Error::Collision)?);
        self.collector
            .reserve_external(allowance)
            .map_err(Error::Collision)?;
        let bytes = cached
            .replay(
                map.id(),
                parent.id,
                cache::Limits {
                    max_memory_bytes: old.checked_add(allowance).ok_or(Error::Limit)?,
                    ..self.limits.cache
                },
            )
            .map_err(Error::Placement)?;
        emit(
            self.sink,
            Context {
                map_id: map.id(),
                original_map_id: parent.id,
                kind: Kind::Tile {
                    x: (cell / 64) as u8,
                    y: (cell % 64) as u8,
                },
                replayed: true,
            },
            &bytes,
            cached.record_count(),
            self.limits,
            &mut self.report,
        )?;
        drop(bytes);
        self.collector
            .release_external(allowance)
            .map_err(Error::Collision)?;
        bump(&mut self.report.replayed_tiles, 1)?;
        if cached.record_count() == 0 {
            bump(&mut self.report.empty_replays, 1)?;
        }
        Ok(())
    }
}
