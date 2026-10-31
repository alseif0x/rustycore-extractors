// SPDX-License-Identifier: GPL-3.0-or-later
//! Strict local RCVGEO1 v8 reader and mutation encoder for synthetic CLI tests.
use forever_casc::{
    collision::{plan::MapSelection, run},
    package::Context,
};

pub const PATH: &str = "vmaps/available-unselected-wmo-geometry-coverage-v8.bin";
pub const COVERAGE_PROFILE: &str = "linux70450-available-unselected-wmo-coverage-v8";
pub const SOURCE_PROFILE: &str = "linux70450-available-unselected-wmo-v1";
pub const DOMAIN_PROFILE: &str = "available-unselected-wmo-v1";
pub const RESOURCE_PROFILE: &str = "linux-collision-request-heavy-resources-v1";

#[derive(Clone, Debug)]
pub struct SectionFact {
    pub ordinal: u32,
    pub records: u32,
    pub copies: u32,
    pub admitted: bool,
}

#[derive(Clone, Debug)]
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
}

#[derive(Clone, Debug)]
pub struct LiquidFact {
    pub file_data_id: u32,
    pub bytes: u64,
    pub magic: [u8; 4],
    pub direct_rows: u64,
    pub copy_records: u64,
    pub materialized_rows: u64,
}

#[derive(Clone, Debug)]
pub struct DomainMapFact {
    pub id: u32,
    pub parent: Option<u32>,
    pub wdt_file_data_id: u32,
    pub depth: u32,
    pub is_parent: bool,
    pub root_absent: bool,
}

#[derive(Clone, Debug)]
pub struct ArtifactFact {
    pub path: String,
    pub bytes: u64,
    pub sha: [u8; 32],
}

#[derive(Clone, Debug)]
pub struct Coverage {
    pub version: u32,
    pub context: Context,
    pub selection: MapSelection,
    pub missing_wdt_policy: run::MissingWdtPolicy,
    pub missing_m2_policy: run::MissingModelPolicy,
    pub missing_m2: Vec<u32>,
    pub missing_wmo_policy: run::MissingWmoPolicy,
    pub missing_wmo: Vec<u32>,
    pub empty_wmo: Vec<u32>,
    pub unselected_wmo: Vec<u32>,
    pub resource_profile: String,
    pub coverage_profile: String,
    pub source_profile: String,
    pub domain_profile: String,
    pub complete_map_source: bool,
    pub complete_global_map_coverage: bool,
    pub gameobject_file_data_id: u32,
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

    fn u32(&mut self) -> Result<u32, &'static str> {
        Ok(u32::from_le_bytes(
            self.take(4)?.try_into().map_err(|_| "coverage word")?,
        ))
    }

    fn u64(&mut self) -> Result<u64, &'static str> {
        Ok(u64::from_le_bytes(
            self.take(8)?.try_into().map_err(|_| "coverage word")?,
        ))
    }

    fn bit(&mut self) -> Result<bool, &'static str> {
        match self.take(1)?[0] {
            0 => Ok(false),
            1 => Ok(true),
            _ => Err("coverage bit"),
        }
    }

    fn text(&mut self) -> Result<&'a str, &'static str> {
        let length = self.u32()? as usize;
        if length > 4096 {
            return Err("coverage text limit");
        }
        std::str::from_utf8(self.take(length)?).map_err(|_| "coverage text")
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
        if reader.take(8)? != b"RCVGEO1\0" {
            return Err("coverage header");
        }
        let version = reader.u32()?;
        if version != 8 {
            return Err("coverage version");
        }
        let context = Context::parse(reader.text()?).map_err(|_| "coverage context")?;
        if context.build != 70450 || context.components != 17 {
            return Err("coverage context scope");
        }
        let selection = MapSelection::All;
        let coverage_profile = reader.text()?.to_owned();
        let source_profile = reader.text()?.to_owned();
        let domain_profile = reader.text()?.to_owned();
        if coverage_profile != COVERAGE_PROFILE
            || source_profile != SOURCE_PROFILE
            || domain_profile != DOMAIN_PROFILE
        {
            return Err("coverage profile");
        }
        if !reader.bit()? {
            return Err("coverage missing-WDT policy");
        }
        let missing_wdt_policy = run::MissingWdtPolicy::DeclareRootAbsent;
        for expected in [
            "excluded-by-profile-v1",
            "placement-only-v1",
            "available-section-map-partial-scope-v1",
        ] {
            if reader.text()? != expected {
                return Err("coverage policy");
            }
        }
        let complete_map_source = reader.bit()?;
        if complete_map_source
            || reader.text()? != "excluded"
            || reader.text()? != "excluded_by_profile"
        {
            return Err("coverage source policy");
        }
        let gameobject_request_made = reader.bit()?;
        if gameobject_request_made
            || reader.text()? != "placement_only"
            || reader.text()? != "complete-LiquidMaterial-LiquidType-v1"
            || reader.text()? != "out_of_scope"
        {
            return Err("coverage exclusion policy");
        }
        let liquid_object_request_made = reader.bit()?;
        let complete_global_map_coverage = reader.bit()?;
        if liquid_object_request_made || complete_global_map_coverage {
            return Err("coverage completion claim");
        }
        let gameobject_file_data_id = reader.u32()?;
        let gameobject_startup_reads = reader.u64()?;
        if gameobject_file_data_id != forever_casc::collision::gameobjects::TABLE_FILE_DATA_ID
            || gameobject_startup_reads != 0
            || reader.u32()? != 0x3ff
        {
            return Err("coverage facts");
        }

        let file_data_id = reader.u32()?;
        let declared_bytes = reader.u64()?;
        let available_bytes = reader.u64()?;
        let full_ckey_verified = reader.bit()?;
        let encoded_key_authenticated = reader.bit()?;
        let declared_direct_rows = reader.u64()?;
        let declared_copy_records = reader.u64()?;
        let direct_rows = reader.u64()?;
        let copy_records = reader.u64()?;
        let materialized_rows = reader.u64()?;
        let section_count = reader.count(16)?;
        let mut sections = Vec::new();
        for _ in 0..section_count {
            sections.push(SectionFact {
                ordinal: reader.u32()?,
                records: reader.u32()?,
                copies: reader.u32()?,
                admitted: reader.bit()?,
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
        };

        let liquid_count = reader.count(2)?;
        if liquid_count != 2 {
            return Err("coverage liquids");
        }
        let mut liquids = Vec::new();
        for _ in 0..liquid_count {
            liquids.push(LiquidFact {
                file_data_id: reader.u32()?,
                bytes: reader.u64()?,
                magic: reader.take(4)?.try_into().map_err(|_| "coverage liquid")?,
                direct_rows: reader.u64()?,
                copy_records: reader.u64()?,
                materialized_rows: reader.u64()?,
            });
        }

        let map_count = reader.count(4096)?;
        let mut maps = Vec::new();
        let mut previous_id = None;
        for _ in 0..map_count {
            let id = reader.u32()?;
            let has_parent = reader.bit()?;
            let parent = reader.u32()?;
            let wdt_file_data_id = reader.u32()?;
            let depth = reader.u32()?;
            let is_parent = reader.bit()?;
            let root_absent = reader.bit()?;
            if !has_parent && parent != 0
                || wdt_file_data_id == 0
                || previous_id.is_some_and(|previous| previous >= id)
            {
                return Err("coverage domain");
            }
            previous_id = Some(id);
            maps.push(DomainMapFact {
                id,
                parent: has_parent.then_some(parent),
                wdt_file_data_id,
                depth,
                is_parent,
                root_absent,
            });
        }

        if !reader.bit()? {
            return Err("coverage missing-model policy");
        }
        let missing_m2_policy = run::MissingModelPolicy::DeclareRootAbsentWmoDoodadM2;
        let missing_count = reader.count(4096)?;
        let mut missing_m2 = Vec::new();
        let mut previous = None;
        for _ in 0..missing_count {
            let id = reader.u32()?;
            if id == 0 || previous.is_some_and(|previous| previous >= id) {
                return Err("coverage missing_m2 order");
            }
            previous = Some(id);
            missing_m2.push(id);
        }
        let resource_profile = reader.text()?.to_owned();
        if resource_profile != RESOURCE_PROFILE {
            return Err("coverage resource profile");
        }
        if !reader.bit()? {
            return Err("coverage missing-WMO policy");
        }
        let missing_wmo_policy =
            run::MissingWmoPolicy::DeclareVacatedAfterRootAbsentM2OrEmptyOrUnselected;
        let missing_wmo_count = reader.count(4096)?;
        let mut missing_wmo = Vec::new();
        let mut previous = None;
        for _ in 0..missing_wmo_count {
            let id = reader.u32()?;
            if id == 0
                || previous.is_some_and(|previous| previous >= id)
                || missing_m2.binary_search(&id).is_ok()
            {
                return Err("coverage missing_wmo order");
            }
            previous = Some(id);
            missing_wmo.push(id);
        }
        let empty_wmo_count = reader.count(4096)?;
        let mut empty_wmo = Vec::new();
        let mut previous = None;
        for _ in 0..empty_wmo_count {
            let id = reader.u32()?;
            if id == 0
                || previous.is_some_and(|previous| previous >= id)
                || missing_m2.binary_search(&id).is_ok()
                || missing_wmo.binary_search(&id).is_ok()
            {
                return Err("coverage empty_wmo order");
            }
            previous = Some(id);
            empty_wmo.push(id);
        }

        let unselected_wmo_count = reader.count(4096)?;
        let mut unselected_wmo = Vec::new();
        let mut previous = None;
        for _ in 0..unselected_wmo_count {
            let id = reader.u32()?;
            if id == 0
                || previous.is_some_and(|previous| previous >= id)
                || missing_m2.binary_search(&id).is_ok()
                || missing_wmo.binary_search(&id).is_ok()
                || empty_wmo.binary_search(&id).is_ok()
            {
                return Err("coverage unselected_wmo order");
            }
            previous = Some(id);
            unselected_wmo.push(id);
        }
        if missing_wmo.iter().any(|id| {
            empty_wmo.binary_search(id).is_ok() || unselected_wmo.binary_search(id).is_ok()
        }) || empty_wmo
            .iter()
            .any(|id| unselected_wmo.binary_search(id).is_ok())
        {
            return Err("coverage WMO omission overlap");
        }

        let artifact_count = reader.count(65_535)?;
        let mut artifacts = Vec::new();
        let mut previous_path: Option<String> = None;
        for _ in 0..artifact_count {
            let path = reader.text()?.to_owned();
            if previous_path
                .as_ref()
                .is_some_and(|previous| previous >= &path)
            {
                return Err("coverage artifact order");
            }
            previous_path = Some(path.clone());
            artifacts.push(ArtifactFact {
                path,
                bytes: reader.u64()?,
                sha: reader.take(32)?.try_into().map_err(|_| "coverage digest")?,
            });
        }
        if reader.at != bytes.len() {
            return Err("coverage trailing bytes");
        }
        Ok(Self {
            version,
            context,
            selection,
            missing_wdt_policy,
            missing_m2_policy,
            missing_m2,
            missing_wmo_policy,
            missing_wmo,
            empty_wmo,
            unselected_wmo,
            resource_profile,
            coverage_profile,
            source_profile,
            domain_profile,
            complete_map_source,
            complete_global_map_coverage,
            gameobject_file_data_id,
            gameobject_request_made,
            gameobject_startup_reads,
            liquid_object_request_made,
            map,
            liquids,
            maps,
            artifacts,
        })
    }

    pub fn encode(&self) -> Vec<u8> {
        let mut writer = Writer { bytes: Vec::new() };
        writer.put(b"RCVGEO1\0");
        writer.u32(self.version);
        writer.text(&self.context.json());
        if let MapSelection::MapAndAncestors(id) = self.selection {
            writer.u32(id);
        }
        writer.text(&self.coverage_profile);
        writer.text(&self.source_profile);
        writer.text(&self.domain_profile);
        if self.version >= 3 {
            writer.bit(self.missing_wdt_policy == run::MissingWdtPolicy::DeclareRootAbsent);
        }
        writer.text("excluded-by-profile-v1");
        writer.text("placement-only-v1");
        writer.text("available-section-map-partial-scope-v1");
        writer.bit(self.complete_map_source);
        writer.text("excluded");
        writer.text("excluded_by_profile");
        writer.bit(self.gameobject_request_made);
        writer.text("placement_only");
        writer.text("complete-LiquidMaterial-LiquidType-v1");
        writer.text("out_of_scope");
        writer.bit(self.liquid_object_request_made);
        writer.bit(self.complete_global_map_coverage);
        writer.u32(self.gameobject_file_data_id);
        writer.u64(self.gameobject_startup_reads);
        writer.u32(0x3ff);
        writer.u32(self.map.file_data_id);
        writer.u64(self.map.declared_bytes);
        writer.u64(self.map.available_bytes);
        writer.bit(self.map.full_ckey_verified);
        writer.bit(self.map.encoded_key_authenticated);
        for value in [
            self.map.declared_direct_rows,
            self.map.declared_copy_records,
            self.map.direct_rows,
            self.map.copy_records,
            self.map.materialized_rows,
        ] {
            writer.u64(value);
        }
        writer.u32(self.map.sections.len() as u32);
        for section in &self.map.sections {
            writer.u32(section.ordinal);
            writer.u32(section.records);
            writer.u32(section.copies);
            writer.bit(section.admitted);
        }
        writer.u32(self.liquids.len() as u32);
        for liquid in &self.liquids {
            writer.u32(liquid.file_data_id);
            writer.u64(liquid.bytes);
            writer.put(&liquid.magic);
            writer.u64(liquid.direct_rows);
            writer.u64(liquid.copy_records);
            writer.u64(liquid.materialized_rows);
        }
        writer.u32(self.maps.len() as u32);
        for map in &self.maps {
            writer.u32(map.id);
            writer.bit(map.parent.is_some());
            writer.u32(map.parent.unwrap_or(0));
            writer.u32(map.wdt_file_data_id);
            writer.u32(map.depth);
            writer.bit(map.is_parent);
            if self.version >= 3 {
                writer.bit(map.root_absent);
            }
        }
        if self.version == 8 {
            writer.bit(
                self.missing_m2_policy == run::MissingModelPolicy::DeclareRootAbsentWmoDoodadM2,
            );
            writer.u32(self.missing_m2.len() as u32);
            for &id in &self.missing_m2 {
                writer.u32(id);
            }
            writer.text(&self.resource_profile);
            writer.bit(
                self.missing_wmo_policy
                    == run::MissingWmoPolicy::DeclareVacatedAfterRootAbsentM2OrEmptyOrUnselected,
            );
            writer.u32(self.missing_wmo.len() as u32);
            for &id in &self.missing_wmo {
                writer.u32(id);
            }
            writer.u32(self.empty_wmo.len() as u32);
            for &id in &self.empty_wmo {
                writer.u32(id);
            }
            writer.u32(self.unselected_wmo.len() as u32);
            for &id in &self.unselected_wmo {
                writer.u32(id);
            }
        }
        writer.u32(self.artifacts.len() as u32);
        for artifact in &self.artifacts {
            writer.text(&artifact.path);
            writer.u64(artifact.bytes);
            writer.put(&artifact.sha);
        }
        writer.bytes
    }
}

struct Writer {
    bytes: Vec<u8>,
}

impl Writer {
    fn put(&mut self, bytes: &[u8]) {
        self.bytes.extend_from_slice(bytes);
    }

    fn u32(&mut self, value: u32) {
        self.put(&value.to_le_bytes());
    }

    fn u64(&mut self, value: u64) {
        self.put(&value.to_le_bytes());
    }

    fn bit(&mut self, value: bool) {
        self.bytes.push(u8::from(value));
    }

    fn text(&mut self, value: &str) {
        self.u32(value.len() as u32);
        self.put(value.as_bytes());
    }
}
