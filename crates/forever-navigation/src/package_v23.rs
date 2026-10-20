// SPDX-License-Identifier: GPL-3.0-or-later
// Added 2026-10-20: selected-navigation package admission API.
//! Linux-only v23 selected-navigation provenance schema and offline package
//! admission. A successful verification establishes the existing selected
//! profile envelope and canonical stored schema only. It does not establish
//! native topology safety, authenticated origin, complete geometry, full-world
//! coverage, or runtime acceptance. Callers must verify again when loading at
//! a later time; this result does not make package files immutable.

mod schema;
#[cfg(test)]
mod tests;
mod verify;

// The public comparison inputs and verification result use these canonical
// envelope types. Consumers need no additional direct acquisition dependency.
pub use forever_casc::{
    Locale,
    package::{Context, Summary},
};

pub use schema::{
    COVERAGE_LIMIT, COVERAGE_PATH, ClosureMap, Coverage, LiquidSnapshot, MAP_LIMIT, MAP_RANGES_MAX,
    MAP_ROWS_MAX, MapSnapshot, PackageIdentity, SourceIdentity, TerrainMap, TileOutput,
    closure_depths, terrain_materialized_rows_match, vmap_materialized_rows_match,
};
pub use verify::{
    Expected, ReadBatch, ReadLimits, RequestedTile, VerifiedNavigation, read_batch, verify,
};

/// A stable product or package diagnostic from v23 schema admission.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Error {
    code: &'static str,
}

impl Error {
    /// Returns the stable diagnostic code used by the existing CLI.
    pub const fn code(self) -> &'static str {
        self.code
    }
}

impl From<&'static str> for Error {
    fn from(code: &'static str) -> Self {
        Self { code }
    }
}

impl From<forever_casc::package::Error> for Error {
    fn from(error: forever_casc::package::Error) -> Self {
        error.code().into()
    }
}

impl std::fmt::Display for Error {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(self.code)
    }
}

impl std::error::Error for Error {}
