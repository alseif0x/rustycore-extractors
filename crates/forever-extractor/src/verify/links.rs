// SPDX-License-Identifier: GPL-3.0-or-later
// Authored 2026-10-13: Linux product cross-artifact facts, without file I/O.
//! Checks only published MAPS/tile-list equality and MMap tile/global presence.
//! No unobserved-map completeness, VMap binding or Detour topology claim.
use super::{Artifact, Component, Error, Plan, Selection, decimal, terrain_map_name};

pub const PROFILE: &str = "forever-terrain-navigation-links-v1";
pub const SCOPE: &str = "artifact_integrity_intrinsic_formats_and_terrain_navigation_links";
pub const MAX_LINK_MAPS: usize = 8192;
pub const LOGICAL_BYTES_PER_MAP: usize = 4096;
pub const MAX_LOGICAL_BYTES: usize = MAX_LINK_MAPS * LOGICAL_BYTES_PER_MAP;
const WORDS: usize = 4096 / u64::BITS as usize;

struct MapFacts {
    id: u32,
    maps: [u64; WORDS],
    listed: [u64; WORDS],
    list: bool,
    navigation_tile: bool,
    navigation_global: bool,
}
impl Default for MapFacts {
    fn default() -> Self {
        Self {
            id: 0,
            maps: [0; WORDS],
            listed: [0; WORDS],
            list: false,
            navigation_tile: false,
            navigation_global: false,
        }
    }
}
// Two fixed 512-byte bitsets and scalar flags fit well inside each 4096-byte
// admission charge, leaving room for collection metadata/allocation overlap.
// This is conservative logical accounting, not an allocator or RSS bound.
const _: () = assert!(std::mem::size_of::<MapFacts>() * 2 <= LOGICAL_BYTES_PER_MAP);

#[derive(Default)]
pub(super) struct Facts {
    maps: Vec<MapFacts>,
}
impl Facts {
    fn map(&mut self, id: u32) -> Result<&mut MapFacts, Error> {
        let index = match self.maps.binary_search_by_key(&id, |map| map.id) {
            Ok(index) => index,
            Err(index) => {
                // Check before reserving or inserting any new map. Reserve no
                // speculative entries beyond the admitted count.
                if self.maps.len() >= MAX_LINK_MAPS {
                    return Err("product_links_map_limit".into());
                }
                self.maps
                    .try_reserve_exact(1)
                    .map_err(|_| Error::from("product_allocation"))?;
                self.maps.insert(
                    index,
                    MapFacts {
                        id,
                        ..Default::default()
                    },
                );
                index
            }
        };
        Ok(&mut self.maps[index])
    }

    /// Caller has already validated canonical names, bounds and intrinsic bytes.
    /// Accumulates facts from that same verified buffer, retaining no file bytes.
    pub(super) fn inspect(&mut self, artifact: &Artifact, bytes: &[u8]) -> Result<(), Error> {
        match artifact.component {
            Component::Terrain => {
                if let Some(name) = artifact
                    .path
                    .strip_prefix("maps/")
                    .and_then(|name| name.strip_suffix(".tilelist"))
                {
                    let map = self.map(decimal(name)?)?;
                    map.list = true;
                    // tilelist::verify has run once in the intrinsic inspector.
                    // Original std::bitset ASCII is highest bit first.
                    for (reverse, &byte) in bytes[12..].iter().enumerate() {
                        if byte == b'1' {
                            set(&mut map.listed, 4095 - reverse);
                        }
                    }
                } else {
                    let (id, x, y) = terrain_map_name(&artifact.path)?;
                    set(&mut self.map(id)?.maps, (y * 64 + x) as usize);
                }
            }
            Component::Navigation => {
                let name = artifact
                    .path
                    .strip_prefix("mmaps/")
                    .ok_or("product_artifact_name")?;
                if let Some(id) = name.strip_suffix(".mmap") {
                    self.map(decimal(id)?)?.navigation_global = true;
                } else {
                    let (id, _) = name.split_once('_').ok_or("product_artifact_name")?;
                    self.map(decimal(id)?)?.navigation_tile = true;
                }
            }
            _ => {}
        }
        Ok(())
    }

    pub(super) fn finish(self, plan: &Plan) -> Result<(), Error> {
        if matches!(plan.terrain, Selection::All | Selection::Map(_)) {
            for map in &self.maps {
                if map.maps.iter().any(|&word| word != 0) && !map.list {
                    return Err(error(
                        "product_links_terrain_missing_list",
                        Component::Terrain,
                    ));
                }
                if map.list && map.maps != map.listed {
                    return Err(error("product_links_terrain_tile_set", Component::Terrain));
                }
            }
        }
        for map in &self.maps {
            if map.navigation_tile && !map.navigation_global {
                return Err(error(
                    "product_links_navigation_missing_global",
                    Component::Navigation,
                ));
            }
        }
        Ok(())
    }
}
fn set(bits: &mut [u64; WORDS], index: usize) {
    bits[index / 64] |= 1u64 << (index % 64);
}
fn error(code: &'static str, component: Component) -> Error {
    Error {
        code,
        component: Some(component),
    }
}

#[cfg(test)]
mod tests;
