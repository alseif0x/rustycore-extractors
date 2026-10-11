// SPDX-License-Identifier: GPL-3.0-or-later
// Authored 2026-10-11: original Rust implementation from pinned format facts.
// No TrinityCore implementation has been copied or imported.
//! Bounded WDT/ADT metadata, terrain projections, sections and MAPS containers.
//! No I/O, authentication, acquisition or runtime acceptance occurs.
//! Callers must supply complete bytes from their separately checked reader.

pub mod adt;
pub mod chunks;
pub mod conversion;
pub mod height;
pub mod liquid;
pub mod liquid_section;
pub mod maps;
pub mod wdt;

pub use adt::{Adt, Cell, HeightProjection, McnkHeader, Mcvt};
pub use chunks::{Chunk, Chunks, Limits};
pub use wdt::{Mphd, Tile, TileFileIds, TileReference, Wdt};

pub const ORACLE_COMMIT: &str = "02245dcd245e7433e524577656177723d3e4992e";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Error {
    Limit,
    Truncated,
    DuplicateChunk,
    MissingChunk,
    InvalidChunkSize,
    UnsupportedVersion,
    UnsupportedReferenceMode,
    MissingTileFileIds,
    InvalidCellCoordinates,
    UnsupportedNestedLayout,
    NonFiniteHeight,
}

impl Error {
    pub const fn code(self) -> &'static str {
        match self {
            Self::Limit => "terrain_policy_limit",
            Self::Truncated => "terrain_chunk_truncated",
            Self::DuplicateChunk => "terrain_duplicate_chunk",
            Self::MissingChunk => "terrain_missing_chunk",
            Self::InvalidChunkSize => "terrain_invalid_chunk_size",
            Self::UnsupportedVersion => "terrain_unsupported_version",
            Self::UnsupportedReferenceMode => "terrain_unsupported_reference_mode",
            Self::MissingTileFileIds => "terrain_missing_tile_file_ids",
            Self::InvalidCellCoordinates => "terrain_invalid_cell_coordinates",
            Self::UnsupportedNestedLayout => "terrain_unsupported_nested_layout",
            Self::NonFiniteHeight => "terrain_nonfinite_height",
        }
    }
}

impl std::fmt::Display for Error {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(self.code())
    }
}

impl std::error::Error for Error {}

#[cfg(test)]
mod tests;
