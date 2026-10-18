// SPDX-License-Identifier: GPL-3.0-or-later
//! Explicit Linux build-70450 available-Map terrain capability. Coverage may
//! remain partial; callback outputs are provisional until the returned
//! completion. See `forever_terrain::conversion::BUILD_70450_FORMAT_ASSUMPTION`.

use super::{available_70374 as shared, plan};
use crate::local_store::LocalStore;

pub use shared::{
    CompatibleMapOutcome, CompatibleSummary, CompletedCompatibleFullMaps, CompletedFullMaps,
    CompletedSelected, Error, Failure, FullMapLimits, Limits, MapFacts, NativeWitness, Sink,
    WdtDisposition,
};

pub const PROFILE: &str = shared::PROFILE_70450;
pub const FULL_MAP_PROFILE: &str = shared::FULL_MAP_PROFILE_70450;
pub const COMPATIBLE_FULL_MAP_PROFILE: &str = shared::COMPATIBLE_FULL_MAP_PROFILE_70450;

/// Complete selected-tile route. It reports Map authentication/coverage facts;
/// successful tile conversions do not imply complete Map or world coverage.
pub fn run_selected(
    store: &mut LocalStore,
    requests: &[plan::TileRequest],
    pack: bool,
    limits: Limits,
    sink: &mut impl Sink,
) -> Result<CompletedSelected, Failure> {
    shared::run_selected_for(
        shared::Target::Build70450,
        store,
        requests,
        pack,
        limits,
        sink,
    )
}

/// Complete each requested map or selected materialized Map domain. Tile
/// selections reject; each emitted list remains provisional until completion.
pub fn run_full_maps(
    store: &mut LocalStore,
    selection: plan::Selection<'_>,
    pack: bool,
    limits: FullMapLimits,
    sink: &mut impl Sink,
) -> Result<CompletedFullMaps, Failure> {
    shared::run_full_maps_for(
        shared::Target::Build70450,
        store,
        selection,
        pack,
        limits,
        sink,
    )
}

/// Compatible whole-map traversal with the same explicit MissingFile-only
/// omission semantics as the build-70374 availability capability.
pub fn run_compatible_full_maps(
    store: &mut LocalStore,
    selection: plan::Selection<'_>,
    pack: bool,
    limits: FullMapLimits,
    sink: &mut impl Sink,
) -> Result<CompletedCompatibleFullMaps, Failure> {
    shared::run_compatible_full_maps_for(
        shared::Target::Build70450,
        store,
        selection,
        pack,
        limits,
        sink,
    )
}
