// SPDX-License-Identifier: GPL-3.0-or-later
// Authored 2026-10-11: original liquid projection from pinned format facts.
// Pin 02245dcd245e7433e524577656177723d3e4992e, wmo.cpp:254-263,315-341,
// 543-557; vmapexport.cpp:312-321,510; Maps/MapDefines.h:111-115.
// No TrinityCore implementation is copied or imported.
//! Borrowed WMO liquid projection. No acquisition, allocation or serialization.

use super::{Error, Group, LiquidView, Root};

/// Closed LiquidType/Material lookup facts supplied by the caller. MaterialID
/// is read with GetUInt8 by the pinned extractor. Missing type or material
/// entries are not ignored liquids. Results must be stable for the projection;
/// header and MLIQ filtering perform separate lookups, as in the source.
pub trait LiquidLookup {
    fn material_id(&self, type_id: u32) -> Option<u8>;
    fn material_flags(&self, material_id: u8) -> Option<u32>;
}

/// Resolved flags/type and a borrowed source view. A filtered liquid retains
/// its mapped type for diagnostics, but does not expose a serializable view.
/// This value has no owned heap storage and does not implement Debug.
#[derive(Clone, Copy)]
pub struct Liquid<'a> {
    flags: u32,
    type_id: u32,
    view: Option<LiquidView<'a>>,
}

impl<'a> Liquid<'a> {
    /// Flag 0x1 denotes MLIQ data; 0x2 denotes an admitted MOGP liquid type.
    /// Fallback from tile nibbles does not retroactively set flag 0x2.
    pub const fn flags(&self) -> u32 {
        self.flags
    }
    pub const fn type_id(&self) -> u32 {
        self.type_id
    }
    pub const fn view(&self) -> Option<LiquidView<'a>> {
        if self.flags & 1 != 0 { self.view } else { None }
    }
}

/// Projects one already parsed group against its root. Framing, finite f32
/// fields and dimensions were validated by Group::parse under caller limits;
/// this transformation neither allocates nor expands those limits. It does not
/// apply group skip predicates or resolve doodads. The caller must use the
/// resolved group context before deciding to serialize this liquid.
pub fn project<'a>(
    root: &Root<'_>,
    group: &Group<'a>,
    lookups: &(impl LiquidLookup + ?Sized),
) -> Result<Liquid<'a>, Error> {
    let header = group.header();
    let mut type_id = if root.header().flags & 4 != 0 {
        legacy_type(header.source_liquid_type, header.flags)
    } else if header.source_liquid_type == 15 {
        0
    } else {
        legacy_type(header.source_liquid_type.wrapping_add(1), header.flags)
    };
    let mut flags = if type_id != 0 && !ignored(type_id, lookups) {
        2
    } else {
        0
    };
    let view = group.liquid();
    if let Some(view) = view {
        flags |= 1;
        if type_id == 0
            && let Some(tile) = view.tiles().iter().find(|tile| **tile & 0x0f != 15)
        {
            type_id = legacy_type(u32::from(*tile & 0x0f) + 1, header.flags);
        }
        // Deliberately includes type zero. A lookup entry for zero may still
        // name a VisualOnly material, suppressing an otherwise empty surface.
        if ignored(type_id, lookups) {
            flags = 0;
        }
    }
    Ok(Liquid {
        flags,
        type_id,
        view,
    })
}

fn ignored(type_id: u32, lookups: &(impl LiquidLookup + ?Sized)) -> bool {
    lookups
        .material_id(type_id)
        .and_then(|material_id| lookups.material_flags(material_id))
        .is_some_and(|flags| flags & 0x2 != 0)
}

fn legacy_type(type_id: u32, flags: u32) -> u32 {
    if (1..21).contains(&type_id) {
        [13 + u32::from(flags & 0x80000 != 0), 14, 19, 20][((type_id - 1) & 3) as usize]
    } else {
        type_id
    }
}

#[cfg(test)]
mod tests;
