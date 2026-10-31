// SPDX-License-Identifier: GPL-3.0-or-later
// Added 2026-10-31: shared available-navigation v57 package schema.
//! Canonical v57 available-navigation provenance and coverage codec.

mod schema;
#[cfg(test)]
mod tests;
mod verify;

pub use crate::package_v23::{Error, ReadLimits};
pub use schema::{
    COVERAGE_MAX, COVERAGE_PATH, Coverage57, MAPS_MAX, MapFact, Output, TILES_MAX, TileOutput, sha,
};
pub use verify::{Expected, ReadBatch, RequestedTile, VerifiedNavigation, read_batch, verify};
pub type ByteParts = (
    Vec<u8>,
    Vec<(crate::tilebuilder::Coordinate, Option<Vec<u8>>)>,
);
