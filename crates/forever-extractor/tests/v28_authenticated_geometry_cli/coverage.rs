// SPDX-License-Identifier: GPL-3.0-or-later
//! Bounded RCVGEO1 reader with authored offsets for canonical tampering tests.
use forever_casc::package::Context;

pub const PATH: &str = "vmaps/selected-geometry-coverage-v1.bin";

#[derive(Debug)]
pub struct SectionFact {
    pub ordinal: u32,
    pub records: u32,
    pub copies: u32,
    pub admitted: bool,
    pub records_offset: usize,
}

#[derive(Debug)]
pub struct MapOffsets {
    pub declared_direct_rows: usize,
    pub materialized_rows: usize,
}

#[derive(Debug)]
pub struct MapFact {
    pub file_data_id: u32,
    pub declared_bytes: u64,
    pub available_bytes: u64,
    pub full_ckey_verified: bool,
    pub encoded_key_authenticated: bool,
    pub declared_direct_rows: u64,
    pub declared_copy_records: u64,
    pub direct_rows: u64,
    pub copy_records: u64,
    pub materialized_rows: u64,
    pub sections: Vec<SectionFact>,
    pub offsets: MapOffsets,
}

#[derive(Debug)]
pub struct LiquidFact {
    pub file_data_id: u32,
    pub direct_rows: u64,
    pub copy_records: u64,
    pub materialized_rows: u64,
}

#[derive(Debug)]
pub struct DomainOffsets {
    pub parent: usize,
    pub wdt: usize,
    pub depth: usize,
    pub is_parent: usize,
}

#[derive(Debug)]
pub struct DomainMapFact {
    pub id: u32,
    pub parent: Option<u32>,
    pub wdt_file_data_id: u32,
    pub depth: u32,
    pub is_parent: bool,
    pub offsets: DomainOffsets,
}

#[derive(Debug)]
pub struct ArtifactFact {
    pub path: String,
    pub bytes: u64,
    pub sha: [u8; 32],
}

#[derive(Debug)]
pub struct Coverage {
    pub context: Context,
    pub requested_map: u32,
    pub complete_map_source: bool,
    pub complete_global_map_coverage: bool,
    pub gameobject_request_made: bool,
    pub gameobject_startup_reads: u64,
    pub liquid_object_request_made: bool,
    pub map: MapFact,
    pub liquids: Vec<LiquidFact>,
    pub maps: Vec<DomainMapFact>,
    pub artifacts: Vec<ArtifactFact>,
}

struct Reader<'a> {
    bytes: &'a [u8],
    at: usize,
}

impl<'a> Reader<'a> {
    fn take(&mut self, count: usize) -> Result<&'a [u8], &'static str> {
        let end = self.at.checked_add(count).ok_or("coverage bounds")?;
        let value = self.bytes.get(self.at..end).ok_or("coverage bounds")?;
        self.at = end;
        Ok(value)
    }

    fn u32_at(&mut self) -> Result<(usize, u32), &'static str> {
        let at = self.at;
        Ok((
            at,
            u32::from_le_bytes(self.take(4)?.try_into().map_err(|_| "coverage word")?),
        ))
    }

    fn u32(&mut self) -> Result<u32, &'static str> {
        self.u32_at().map(|(_, value)| value)
    }

    fn u64_at(&mut self) -> Result<(usize, u64), &'static str> {
        let at = self.at;
        Ok((
            at,
            u64::from_le_bytes(self.take(8)?.try_into().map_err(|_| "coverage word")?),
        ))
    }

    fn u64(&mut self) -> Result<u64, &'static str> {
        self.u64_at().map(|(_, value)| value)
    }

    fn bit_at(&mut self) -> Result<(usize, bool), &'static str> {
        let at = self.at;
        match self.take(1)?[0] {
            0 => Ok((at, false)),
            1 => Ok((at, true)),
            _ => Err("coverage bit"),
        }
    }

    fn bit(&mut self) -> Result<bool, &'static str> {
        self.bit_at().map(|(_, value)| value)
    }

    fn text(&mut self) -> Result<&'a str, &'static str> {
        let length = self.u32()? as usize;
        if length > 4096 {
            return Err("coverage text limit");
        }
        std::str::from_utf8(self.take(length)?).map_err(|_| "coverage text")
    }

    fn expect_text(&mut self, expected: &str) -> Result<(), &'static str> {
        if self.text()? == expected {
            Ok(())
        } else {
            Err("coverage profile")
        }
    }

    fn count(&mut self, maximum: usize) -> Result<usize, &'static str> {
        let count = self.u32()? as usize;
        if count > maximum {
            Err("coverage count")
        } else {
            Ok(count)
        }
    }
}

impl Coverage {
    pub fn parse(bytes: &[u8]) -> Result<Self, &'static str> {
        if bytes.len() > 128 * 1024 * 1024 {
            return Err("coverage size");
        }
        let mut reader = Reader { bytes, at: 0 };
        if reader.take(8)? != b"RCVGEO1\0" || reader.u32()? != 1 {
            return Err("coverage header");
        }
        let context = Context::parse(reader.text()?).map_err(|_| "coverage context")?;
        if context.build != 70450 || context.components != 17 {
            return Err("coverage context scope");
        }
        let requested_map = reader.u32()?;
        reader.expect_text("linux70450-selected-map-geometry-coverage-v1")?;
        reader.expect_text("linux70450-selected-map-geometry-v1")?;
        reader.expect_text("available-map-active-ancestors-v1")?;
        reader.expect_text("excluded-by-profile-v1")?;
        reader.expect_text("placement-only-v1")?;
        reader.expect_text("available-section-map-partial-scope-v1")?;
        let complete_map_source = reader.bit()?;
        reader.expect_text("excluded")?;
        reader.expect_text("excluded_by_profile")?;
        let gameobject_request_made = reader.bit()?;
        reader.expect_text("placement_only")?;
        reader.expect_text("complete-LiquidMaterial-LiquidType-v1")?;
        reader.expect_text("out_of_scope")?;
        let liquid_object_request_made = reader.bit()?;
        let complete_global_map_coverage = reader.bit()?;
        if reader.u32()? != forever_casc::collision::gameobjects::TABLE_FILE_DATA_ID {
            return Err("coverage gameobject identity");
        }
        let gameobject_startup_reads = reader.u64()?;
        if reader.u32()? != 0x3ff {
            return Err("coverage facts");
        }

        let file_data_id = reader.u32()?;
        let declared_bytes = reader.u64()?;
        let available_bytes = reader.u64()?;
        let full_ckey_verified = reader.bit()?;
        let encoded_key_authenticated = reader.bit()?;
        let (declared_direct_rows_offset, declared_direct_rows) = reader.u64_at()?;
        let declared_copy_records = reader.u64()?;
        let direct_rows = reader.u64()?;
        let copy_records = reader.u64()?;
        let (materialized_rows_offset, materialized_rows) = reader.u64_at()?;
        let section_count = reader.count(16)?;
        let mut sections = Vec::new();
        for _ in 0..section_count {
            let ordinal = reader.u32()?;
            let (records_offset, records) = reader.u32_at()?;
            let copies = reader.u32()?;
            let admitted = reader.bit()?;
            sections.push(SectionFact {
                ordinal,
                records,
                copies,
                admitted,
                records_offset,
            });
        }
        let map = MapFact {
            file_data_id,
            declared_bytes,
            available_bytes,
            full_ckey_verified,
            encoded_key_authenticated,
            declared_direct_rows,
            declared_copy_records,
            direct_rows,
            copy_records,
            materialized_rows,
            sections,
            offsets: MapOffsets {
                declared_direct_rows: declared_direct_rows_offset,
                materialized_rows: materialized_rows_offset,
            },
        };

        let liquid_count = reader.count(2)?;
        if liquid_count != 2 {
            return Err("coverage liquids");
        }
        let mut liquids = Vec::new();
        for _ in 0..liquid_count {
            let file_data_id = reader.u32()?;
            let _bytes = reader.u64()?;
            let _magic = reader.take(4)?;
            let direct_rows = reader.u64()?;
            let copy_records = reader.u64()?;
            let materialized_rows = reader.u64()?;
            liquids.push(LiquidFact {
                file_data_id,
                direct_rows,
                copy_records,
                materialized_rows,
            });
        }

        let map_count = reader.count(4096)?;
        let mut maps = Vec::new();
        for _ in 0..map_count {
            let id = reader.u32()?;
            let has_parent = reader.bit()?;
            let (parent_offset, parent) = reader.u32_at()?;
            let (wdt_offset, wdt_file_data_id) = reader.u32_at()?;
            let (depth_offset, depth) = reader.u32_at()?;
            let (is_parent_offset, is_parent) = reader.bit_at()?;
            if !has_parent && parent != 0 {
                return Err("coverage parent");
            }
            maps.push(DomainMapFact {
                id,
                parent: has_parent.then_some(parent),
                wdt_file_data_id,
                depth,
                is_parent,
                offsets: DomainOffsets {
                    parent: parent_offset,
                    wdt: wdt_offset,
                    depth: depth_offset,
                    is_parent: is_parent_offset,
                },
            });
        }

        let artifact_count = reader.count(65_535)?;
        let mut artifacts = Vec::new();
        for _ in 0..artifact_count {
            let path = reader.text()?.to_owned();
            let bytes = reader.u64()?;
            let sha = reader.take(32)?.try_into().map_err(|_| "coverage digest")?;
            artifacts.push(ArtifactFact { path, bytes, sha });
        }
        if reader.at != bytes.len() {
            return Err("coverage trailing bytes");
        }
        Ok(Self {
            context,
            requested_map,
            complete_map_source,
            complete_global_map_coverage,
            gameobject_request_made,
            gameobject_startup_reads,
            liquid_object_request_made,
            map,
            liquids,
            maps,
            artifacts,
        })
    }
}

pub fn put_u32(bytes: &mut [u8], at: usize, value: u32) {
    bytes[at..at + 4].copy_from_slice(&value.to_le_bytes());
}

pub fn put_u64(bytes: &mut [u8], at: usize, value: u64) {
    bytes[at..at + 8].copy_from_slice(&value.to_le_bytes());
}

pub fn put_bool(bytes: &mut [u8], at: usize, value: bool) {
    bytes[at] = u8::from(value);
}
