// SPDX-License-Identifier: GPL-3.0-or-later
// Authored 2026-10-11: original checked WDT metadata reader from format facts.
//! Pinned sources: map_extractor/wdt.h:25-86 and System.cpp:1104-1125;
//! vmap4_extractor/wdtfile.h:31-65 and wdtfile.cpp:65-79 confirm exact sizes.
//! Only MAIN bit 0 and MPHD bit 0x200 acquire meaning here. All other values
//! are retained. Neither global-WMO semantics nor ADT payloads are parsed.

use crate::{Chunks, Error, Limits};

pub const MAP_SIZE: usize = 64;
pub const TILE_COUNT: usize = MAP_SIZE * MAP_SIZE;
pub const FILE_VERSION: u32 = 18;
pub const FILE_ID_REFERENCES: u32 = 0x200;
pub const TILE_PRESENT: u32 = 1;
const MAIN_BYTES: usize = TILE_COUNT * 8;
const MAID_BYTES: usize = TILE_COUNT * 32;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Mphd {
    pub flags: u32,
    pub lgt_file_data_id: u32,
    pub occ_file_data_id: u32,
    pub fogs_file_data_id: u32,
    pub mpv_file_data_id: u32,
    pub tex_file_data_id: u32,
    pub wdl_file_data_id: u32,
    pub pd4_file_data_id: u32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TileFileIds {
    pub root_adt: u32,
    pub obj0_adt: u32,
    pub obj1_adt: u32,
    pub tex0_adt: u32,
    pub lod_adt: u32,
    pub map_texture: u32,
    pub map_texture_n: u32,
    pub minimap_texture: u32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Tile {
    /// WDT array position [y][x], not the later MAPS grid naming order.
    pub x: u8,
    pub y: u8,
    pub flags: u32,
    /// The second MAIN word is data1/AsyncId; no semantics are inferred.
    pub data1: u32,
    pub file_ids: Option<TileFileIds>,
}
impl Tile {
    pub const fn present(self) -> bool {
        self.flags & TILE_PRESENT != 0
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TileReference {
    pub x: u8,
    pub y: u8,
    pub file_data_id: u32,
}

pub struct Wdt<'a> {
    chunks: Chunks<'a>,
    mphd: Option<Mphd>,
    main: &'a [u8],
    maid: Option<&'a [u8]>,
}

impl<'a> Wdt<'a> {
    pub fn parse(bytes: &'a [u8], limits: Limits) -> Result<Self, Error> {
        let chunks = Chunks::parse(bytes, limits)?;
        let version = required(&chunks, *b"MVER", 4)?;
        if word(version, 0) != FILE_VERSION {
            return Err(Error::UnsupportedVersion);
        }
        let main = required(&chunks, *b"MAIN", MAIN_BYTES)?;
        let mphd = optional(&chunks, *b"MPHD", 32)?.map(|payload| Mphd {
            flags: word(payload, 0),
            lgt_file_data_id: word(payload, 4),
            occ_file_data_id: word(payload, 8),
            fogs_file_data_id: word(payload, 12),
            mpv_file_data_id: word(payload, 16),
            tex_file_data_id: word(payload, 20),
            wdl_file_data_id: word(payload, 24),
            pd4_file_data_id: word(payload, 28),
        });
        let maid = optional(&chunks, *b"MAID", MAID_BYTES)?;
        if mphd.is_some_and(|header| header.flags & FILE_ID_REFERENCES != 0)
            && maid.is_none()
            && main
                .as_chunks::<8>()
                .0
                .iter()
                .any(|entry| word(entry, 0) & TILE_PRESENT != 0)
        {
            return Err(Error::MissingTileFileIds);
        }
        Ok(Self {
            chunks,
            mphd,
            main,
            maid,
        })
    }

    pub const fn chunks(&self) -> &Chunks<'a> {
        &self.chunks
    }
    pub const fn mphd(&self) -> Option<Mphd> {
        self.mphd
    }
    pub const fn has_maid(&self) -> bool {
        self.maid.is_some()
    }

    pub fn tile(&self, x: usize, y: usize) -> Option<Tile> {
        if x >= MAP_SIZE || y >= MAP_SIZE {
            return None;
        }
        Some(self.tile_at(y * MAP_SIZE + x))
    }

    /// Physical order: y outer, x inner, exactly System.cpp:1109-1111.
    pub fn tiles(&self) -> impl ExactSizeIterator<Item = Tile> + '_ {
        (0..TILE_COUNT).map(|index| self.tile_at(index))
    }

    /// Return each selected coordinate with its rootADT, retaining duplicate
    /// and zero IDs. This is a dependency projection, not a read or success
    /// claim. A missing MPHD/clear 0x200 is an explicit unsupported name mode.
    pub fn modern_root_adts(&self) -> Result<impl Iterator<Item = TileReference> + '_, Error> {
        if !self
            .mphd
            .is_some_and(|header| header.flags & FILE_ID_REFERENCES != 0)
        {
            return Err(Error::UnsupportedReferenceMode);
        }
        // parse() guarantees MAID for every present tile in this mode. If it
        // is absent, MAIN is empty and this iterator legitimately has no rows.
        Ok(self.tiles().filter_map(|tile| {
            if !tile.present() {
                return None;
            }
            tile.file_ids.map(|ids| TileReference {
                x: tile.x,
                y: tile.y,
                file_data_id: ids.root_adt,
            })
        }))
    }

    fn tile_at(&self, index: usize) -> Tile {
        let offset = index * 8;
        let file_ids = self.maid.map(|maid| {
            let offset = index * 32;
            TileFileIds {
                root_adt: word(maid, offset),
                obj0_adt: word(maid, offset + 4),
                obj1_adt: word(maid, offset + 8),
                tex0_adt: word(maid, offset + 12),
                lod_adt: word(maid, offset + 16),
                map_texture: word(maid, offset + 20),
                map_texture_n: word(maid, offset + 24),
                minimap_texture: word(maid, offset + 28),
            }
        });
        Tile {
            x: (index % MAP_SIZE) as u8,
            y: (index / MAP_SIZE) as u8,
            flags: word(self.main, offset),
            data1: word(self.main, offset + 4),
            file_ids,
        }
    }
}

fn required<'a>(chunks: &Chunks<'a>, tag: [u8; 4], size: usize) -> Result<&'a [u8], Error> {
    optional(chunks, tag, size)?.ok_or(Error::MissingChunk)
}
fn optional<'a>(chunks: &Chunks<'a>, tag: [u8; 4], size: usize) -> Result<Option<&'a [u8]>, Error> {
    let Some(chunk) = chunks.unique(tag)? else {
        return Ok(None);
    };
    if chunk.payload().len() != size {
        return Err(Error::InvalidChunkSize);
    }
    Ok(Some(chunk.payload()))
}
// All positions are bounded by the exact reviewed chunk sizes and tile range.
fn word(bytes: &[u8], offset: usize) -> u32 {
    u32::from_le_bytes([
        bytes[offset],
        bytes[offset + 1],
        bytes[offset + 2],
        bytes[offset + 3],
    ])
}
