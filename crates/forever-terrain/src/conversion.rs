// SPDX-License-Identifier: GPL-3.0-or-later
// Authored 2026-10-11: original composition of bounded terrain components.
//! Converts one complete, already acquired ADT into a verified MAPS v10 object.
//! Pinned consumer facts: System.cpp:478-1031,1056-1080,1104-1125 and
//! loadlib.cpp:87-97,109-122 at crate::ORACLE_COMMIT. No upstream code imported.
//!
//! The caller supplies authenticated complete bytes and a deterministic liquid
//! lookup with its copy semantics already resolved. This API performs no I/O,
//! WDT selection, DB2 loading, acquisition, naming or runtime integration.
//! Context is a caller declaration, not proof of the ADT's map/build identity.
//! Missing cells remain zero and missing MCVT uses the cell base, as in the pin;
//! coverage statistics distinguish those cases from full observed coverage.

use crate::{Adt, adt::CELL_COUNT, height, liquid, liquid_section, maps};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Context {
    pub map_id: u32,
    /// WDT row, passed first to the pinned deep-water exception predicate.
    pub tile_y: u8,
    /// WDT column, passed second to the pinned deep-water exception predicate.
    pub tile_x: u8,
    pub build: u32,
    /// The same source switch controls both MHGT and MLIQ packing decisions.
    pub pack: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Limits {
    pub max_input_bytes: usize,
    pub max_chunks: usize,
    pub max_output_bytes: usize,
}
impl Default for Limits {
    fn default() -> Self {
        let input = crate::Limits::default();
        Self {
            max_input_bytes: input.max_file_bytes,
            max_chunks: input.max_chunks,
            max_output_bytes: maps::MAX_MAP_BYTES,
        }
    }
}
impl Limits {
    fn validate(self) -> Result<(), Error> {
        let ceiling = Self::default();
        if self.max_input_bytes > ceiling.max_input_bytes
            || self.max_chunks > ceiling.max_chunks
            || self.max_output_bytes > ceiling.max_output_bytes
        {
            return Err(Error::InvalidLimits);
        }
        Ok(())
    }
}

/// Counts and presence only; no source payload, path or table contents.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Stats {
    /// Source [y][x] cell order. Repeated coordinates set the same slot.
    pub coverage: [bool; CELL_COUNT],
    pub visited_cells: usize,
    pub unique_cells: usize,
    /// Counts physical MCNK records without MCVT, including duplicates.
    pub missing_mcvt: usize,
    pub has_flight_bounds: bool,
    pub has_liquid: bool,
    /// Source sticky presence, even if a later overwrite cleared every bit.
    pub has_holes: bool,
    pub ignore_deep_water: bool,
}

/// Complete output only. Deliberately no Debug implementation for raw bytes.
pub struct ConvertedTile {
    bytes: Vec<u8>,
    stats: Stats,
}
impl ConvertedTile {
    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }
    pub fn into_bytes(self) -> Vec<u8> {
        self.bytes
    }
    pub const fn stats(&self) -> &Stats {
        &self.stats
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Error {
    InvalidCoordinates,
    InvalidLimits,
    Adt(crate::Error),
    Height(height::Error),
    Liquid(liquid::Error),
    LiquidSection(liquid_section::Error),
    Maps(maps::Error),
}
impl Error {
    pub const fn code(self) -> &'static str {
        match self {
            Self::InvalidCoordinates => "conversion_invalid_coordinates",
            Self::InvalidLimits => "conversion_invalid_limits",
            Self::Adt(error) => error.code(),
            Self::Height(error) => error.code(),
            Self::Liquid(error) => error.code(),
            Self::LiquidSection(error) => error.code(),
            Self::Maps(error) => error.code(),
        }
    }
}
impl std::fmt::Display for Error {
    fn fmt(&self, out: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        out.write_str(self.code())
    }
}
impl std::error::Error for Error {}

/// Limits may reduce the 16 MiB / 4096-frame / 202112-byte ceilings, never
/// expand them. Zero limits are valid reductions and reject any required data.
/// Top-level and direct child frames share the parser's aggregate frame cap.
/// Projections have fixed grid sizes; output sections and final allocation are
/// bounded independently. No partial output or statistics escape on error.
pub fn convert_adt(
    bytes: &[u8],
    lookup: &impl liquid::LiquidLookup,
    context: Context,
    limits: Limits,
) -> Result<ConvertedTile, Error> {
    convert_impl(bytes, lookup, context, limits, false)
}

/// Explicit70374 complete-ADT conversion. Caller binds acquisition/Map coverage.
/// This does not widen the historical convert_adt build gate.
pub fn convert_adt_70374(
    bytes: &[u8],
    lookup: &impl liquid::LiquidLookup,
    context: Context,
    limits: Limits,
) -> Result<ConvertedTile, Error> {
    convert_impl(bytes, lookup, context, limits, true)
}

fn convert_impl(
    bytes: &[u8],
    lookup: &impl liquid::LiquidLookup,
    context: Context,
    limits: Limits,
    current: bool,
) -> Result<ConvertedTile, Error> {
    limits.validate()?;
    if usize::from(context.tile_x) >= crate::wdt::MAP_SIZE
        || usize::from(context.tile_y) >= crate::wdt::MAP_SIZE
    {
        return Err(Error::InvalidCoordinates);
    }
    if if current {
        context.build != 70374
    } else {
        !matches!(context.build, 70170 | 70205)
    } {
        return Err(Error::Maps(maps::Error::UnsupportedBuild));
    }
    let adt = Adt::parse(
        bytes,
        crate::Limits {
            max_file_bytes: limits.max_input_bytes,
            max_chunks: limits.max_chunks,
        },
    )
    .map_err(Error::Adt)?;
    // System.cpp:716-723 selects the unique top-level chunk by tag. MHDR
    // flags/offsets are not consulted. Duplicate MFBO and non-exact payloads
    // retain the components' documented stricter admission policy.
    let flight = adt
        .chunks()
        .unique(*b"MFBO")
        .map_err(Error::Adt)?
        .map(|chunk| height::FlightBounds::parse(chunk.payload()))
        .transpose()
        .map_err(Error::Height)?;
    let projection = adt.project_heights().map_err(Error::Adt)?;
    let area = maps::AreaSection::from_projection(&projection);
    let height = height::HeightSection::from_projection(
        &projection,
        height::Options {
            pack_integers: context.pack,
        },
        flight.as_ref(),
        limits.max_output_bytes.min(height::MAX_SECTION_BYTES),
    )
    .map_err(Error::Height)?;
    let ignore_deep_water = liquid::ignore_deep_water(
        context.map_id,
        u32::from(context.tile_y),
        u32::from(context.tile_x),
    );
    // Liquid heights come from their own raw projection, never from the
    // clamped/quantized terrain height section above.
    let liquid_projection =
        liquid::project_liquids(&adt, lookup, ignore_deep_water).map_err(Error::Liquid)?;
    let liquid = liquid_section::LiquidSection::from_projection(
        &liquid_projection,
        liquid_section::Options {
            pack_integers: context.pack,
        },
        limits
            .max_output_bytes
            .min(liquid_section::MAX_SECTION_BYTES),
    )
    .map_err(Error::LiquidSection)?;
    let holes = if projection.has_holes() {
        Some(
            projection
                .hole_bytes()
                .try_into()
                .map_err(|_| Error::Maps(maps::Error::Holes))?,
        )
    } else {
        None
    };
    let assemble = if current {
        maps::assemble_70374
    } else {
        maps::assemble
    };
    let output = assemble(
        context.build,
        maps::MapSections {
            area: &area,
            height: &height,
            liquid: liquid.as_ref(),
            holes,
        },
        limits.max_output_bytes,
    )
    .map_err(Error::Maps)?;
    let stats = Stats {
        coverage: *projection.coverage(),
        visited_cells: projection.visited_cells(),
        unique_cells: projection.unique_cells(),
        missing_mcvt: adt
            .cells()
            .iter()
            .filter(|cell| cell.mcvt().is_none())
            .count(),
        has_flight_bounds: flight.is_some(),
        has_liquid: liquid.is_some(),
        has_holes: projection.has_holes(),
        ignore_deep_water,
    };
    Ok(ConvertedTile {
        bytes: output,
        stats,
    })
}

#[cfg(test)]
mod tests;
