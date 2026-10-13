// SPDX-License-Identifier: GPL-3.0-or-later
// Authored 2026-10-11. Facts: pinned wmo.cpp:46-167 and wmo.h:54-94.
use super::{
    Chunks, Error, Limits, bits3, bounds, chunks, cstring, exact, finite, once, records, scan,
    short, word,
};

#[derive(Clone, Copy)]
pub struct RootHeader {
    pub textures: u32,
    pub groups: u32,
    pub portals: u32,
    pub lights: u32,
    pub doodad_names: u32,
    pub doodad_definitions: u32,
    pub doodad_sets: u32,
    pub color: u32,
    pub root_wmo_id: u32,
    pub bounds_bits: [u32; 6],
    pub flags: u16,
    pub num_lod: u16,
}

pub struct DoodadSet<'a> {
    pub name_bytes: &'a [u8],
    pub start_index: u32,
    pub count: u32,
    pub padding: &'a [u8],
}
#[derive(Clone, Copy)]
pub struct DoodadSpawn {
    pub name_index: u32,
    pub name_flags: u8,
    pub position_bits: [u32; 3],
    pub rotation_bits: [u32; 4],
    pub scale_bits: u32,
    pub color: u32,
}

/// No model acquisition or ValidDoodadNames set is inferred. MODS/MODD header
/// counts are retained, not used to truncate complete chunk records. Set spans
/// must fit MODD. MODN and MODI are mutually exclusive, matching the assertion
/// in the pin. MODN names are bytes, not assumed UTF-8 or local filesystem paths.
/// GFID must follow MOHD: the native reader consumes it with the group count
/// already seen, rather than interpreting it with a later header.
pub struct Root<'a> {
    bytes: &'a [u8],
    header: RootHeader,
    version: Option<u32>,
    names: &'a [u8],
    gfid: Option<&'a [u8]>,
    lod_count: usize,
    sets: &'a [u8],
    spawns: &'a [u8],
    doodad_names: Option<&'a [u8]>,
    doodad_ids: Option<&'a [u8]>,
    chunk_count: usize,
}

impl<'a> Root<'a> {
    pub fn parse(bytes: &'a [u8], limits: Limits) -> Result<Self, Error> {
        limits.input(bytes)?;
        let (mut head, mut version, mut names, mut gfid) = (None, None, None, None);
        let (mut sets, mut spawns, mut paths, mut ids) = (None, None, None, None);
        let mut count = 0;
        scan(bytes, &mut count, limits, |chunk| {
            if &chunk.tag == b"GFID" && head.is_none() {
                return Err(Error::Unsupported);
            }
            let slot = match &chunk.tag {
                b"MOHD" => &mut head,
                b"MVER" => &mut version,
                b"MOGN" => &mut names,
                b"GFID" => &mut gfid,
                b"MODS" => &mut sets,
                b"MODD" => &mut spawns,
                b"MODN" => &mut paths,
                b"MODI" => &mut ids,
                _ => return Ok(()),
            };
            once(slot, chunk.payload)
        })?;
        let h = head.ok_or(Error::MissingHeader)?;
        exact(h, 64)?;
        let header = RootHeader {
            textures: word(h, 0)?,
            groups: word(h, 4)?,
            portals: word(h, 8)?,
            lights: word(h, 12)?,
            doodad_names: word(h, 16)?,
            doodad_definitions: word(h, 20)?,
            doodad_sets: word(h, 24)?,
            color: word(h, 28)?,
            root_wmo_id: word(h, 32)?,
            bounds_bits: bounds(h, 36)?,
            flags: short(h, 60)?,
            num_lod: short(h, 62)?,
        };
        if header.groups as usize > limits.max_groups {
            return Err(Error::Limit);
        }
        let version = version
            .map(|v| {
                exact(v, 4)?;
                word(v, 0)
            })
            .transpose()?;
        let lod_count = if header.flags & 0x10 == 0 {
            1
        } else if header.num_lod == 0 {
            3
        } else {
            usize::from(header.num_lod)
        };
        if let Some(gfid) = gfid {
            let expected = (header.groups as usize)
                .checked_mul(lod_count)
                .and_then(|v| v.checked_mul(4))
                .ok_or(Error::Bounds)?;
            exact(gfid, expected)?;
        }
        if paths.is_some() && ids.is_some() {
            return Err(Error::Unsupported);
        }
        if let Some(ids) = ids {
            records(ids, 4, limits.max_metadata_records)?;
        }
        if let Some(paths) = paths {
            let mut at = 0;
            let mut names = 0;
            while at < paths.len() {
                if names >= limits.max_metadata_records {
                    return Err(Error::Limit);
                }
                at += cstring(paths, at)?.len() + 1;
                names += 1;
            }
        }
        let sets = sets.unwrap_or(&[]);
        let spawns = spawns.unwrap_or(&[]);
        records(sets, 32, limits.max_metadata_records)?;
        let spawn_count = records(spawns, 40, limits.max_metadata_records)?;
        for set in sets.as_chunks::<32>().0 {
            let end = (word(set, 20)? as usize)
                .checked_add(word(set, 24)? as usize)
                .ok_or(Error::Bounds)?;
            if end > spawn_count {
                return Err(Error::Bounds);
            }
        }
        for spawn in spawns.as_chunks::<40>().0 {
            bits3(spawn, 4)?;
            for at in [16, 20, 24, 28, 32] {
                finite(word(spawn, at)?)?;
            }
            // Name references are retained structurally. Native validity also
            // depends on successful model acquisition, outside this module.
        }
        Ok(Self {
            bytes,
            header,
            version,
            names: names.unwrap_or(&[]),
            gfid,
            lod_count,
            sets,
            spawns,
            doodad_names: paths,
            doodad_ids: ids,
            chunk_count: count,
        })
    }

    pub const fn header(&self) -> RootHeader {
        self.header
    }
    pub const fn version(&self) -> Option<u32> {
        self.version
    }
    pub const fn bytes(&self) -> &'a [u8] {
        self.bytes
    }
    pub fn chunks(&self) -> Chunks<'a> {
        chunks(self.bytes)
    }
    pub const fn chunk_count(&self) -> usize {
        self.chunk_count
    }
    pub const fn group_names(&self) -> &'a [u8] {
        self.names
    }
    pub const fn has_gfid(&self) -> bool {
        self.gfid.is_some()
    }
    pub const fn declared_lod_count(&self) -> usize {
        self.lod_count
    }
    /// Complete rectangular GFID inventory, including zero entries and LODs
    /// unused by the C++ caller. Exact declared shape is a stricter policy.
    pub fn group_file_data_id(&self, lod: usize, group: usize) -> Option<u32> {
        let gfid = self.gfid?;
        if lod >= self.lod_count || group >= self.header.groups as usize {
            return None;
        }
        let at = lod
            .checked_mul(self.header.groups as usize)?
            .checked_add(group)?
            .checked_mul(4)?;
        word(gfid, at).ok()
    }
    /// Native acquisition order: LOD zero only, zero IDs omitted, duplicates
    /// retained. No fallback to group names when GFID is absent.
    pub fn detailed_group_ids(&self) -> impl Iterator<Item = u32> + '_ {
        (0..self.header.groups as usize)
            .filter_map(|i| self.group_file_data_id(0, i))
            .filter(|id| *id != 0)
    }
    pub fn doodad_set_count(&self) -> usize {
        self.sets.len() / 32
    }
    pub fn doodad_set(&self, index: usize) -> Option<DoodadSet<'a>> {
        let at = index.checked_mul(32)?;
        let bytes = self.sets.get(at..at.checked_add(32)?)?;
        Some(DoodadSet {
            name_bytes: &bytes[..20],
            start_index: word(bytes, 20).ok()?,
            count: word(bytes, 24).ok()?,
            padding: &bytes[28..],
        })
    }
    pub fn doodad_spawn_count(&self) -> usize {
        self.spawns.len() / 40
    }
    pub fn doodad_spawn(&self, index: usize) -> Option<DoodadSpawn> {
        let at = index.checked_mul(40)?;
        let bytes = self.spawns.get(at..at.checked_add(40)?)?;
        let name = word(bytes, 0).ok()?;
        Some(DoodadSpawn {
            name_index: name & 0x00ff_ffff,
            name_flags: (name >> 24) as u8,
            position_bits: bits3(bytes, 4).ok()?,
            rotation_bits: [
                word(bytes, 16).ok()?,
                word(bytes, 20).ok()?,
                word(bytes, 24).ok()?,
                word(bytes, 28).ok()?,
            ],
            scale_bits: word(bytes, 32).ok()?,
            color: word(bytes, 36).ok()?,
        })
    }
    pub const fn doodad_name_bytes(&self) -> Option<&'a [u8]> {
        self.doodad_names
    }
    pub fn doodad_file_data_id_count(&self) -> usize {
        self.doodad_ids.map_or(0, |v| v.len() / 4)
    }
    pub fn doodad_file_data_id(&self, index: usize) -> Option<u32> {
        word(self.doodad_ids?, index.checked_mul(4)?).ok()
    }
}
