// SPDX-License-Identifier: GPL-3.0-or-later
// Authored by RustyCore Extractors, 2026-10-17.
//! Versioned producer-reported coverage and complete association witnesses.
//! Available/unknown sections remain distinct from complete liquid sources.
//! This metadata authenticates declared output associations, not unobserved
//! client data, per-model omission reasons or collision runtime compatibility.
use crate::Error;
use forever_casc::{
    collision::{
        available_70374,
        compatible::{self, startup},
        gameobjects,
        run::compatible_inventory::coverage,
    },
    integrity,
    package::{Artifact, Component, Context},
};
pub(crate) const PATH: &str = "vmaps/compatible-collision-coverage-v1.bin";
pub(crate) const MAGIC: &[u8; 8] = b"RCVMCV1\0";
pub(crate) const PROOF_MAGIC: &[u8; 8] = b"RCVMPR1\0";
pub(crate) const MAX_BYTES: usize = 128 * 1024 * 1024;
pub(crate) const MAX_MAPS: usize = 4096;
pub(crate) const MAX_FILES: usize = 262144;
pub(crate) const HEAP: usize = MAX_BYTES + MAX_MAPS * size_of::<u32>() + 4096;
const LIMIT: &str = "product_collision_coverage_limit";
const SCHEMA: &str = "product_collision_coverage_schema";
const fn placement_max_record() -> usize {
    62 + forever_vmap::placement::MAX_NAME_BYTES
}
/// One allocation before acquisition. No implicit Vec growth is permitted.
pub(crate) struct Encoder {
    bytes: Vec<u8>,
    count_at: usize,
    files: usize,
    total: u64,
}
impl Encoder {
    pub(crate) fn new() -> Result<Self, Error> {
        let mut bytes = Vec::new();
        bytes
            .try_reserve_exact(MAX_BYTES)
            .map_err(|_| "product_allocation")?;
        if bytes.capacity() > MAX_BYTES {
            return Err(LIMIT.into());
        }
        Ok(Self {
            bytes,
            count_at: 0,
            files: 0,
            total: 0,
        })
    }
    fn put(&mut self, b: &[u8]) -> Result<(), Error> {
        if b.len() > self.bytes.capacity() - self.bytes.len() {
            return Err(LIMIT.into());
        }
        self.bytes.extend_from_slice(b);
        Ok(())
    }
    fn n(&mut self, n: usize) -> Result<(), Error> {
        self.u64(u64::try_from(n).map_err(|_| LIMIT)?)
    }
    fn u64(&mut self, n: u64) -> Result<(), Error> {
        self.put(&n.to_le_bytes())
    }
    fn u32(&mut self, n: u32) -> Result<(), Error> {
        self.put(&n.to_le_bytes())
    }
    fn bit(&mut self, b: bool) -> Result<(), Error> {
        self.put(&[u8::from(b)])
    }
    fn opt(&mut self, n: Option<u32>) -> Result<(), Error> {
        self.bit(n.is_some())?;
        self.u32(n.unwrap_or(0))
    }
    fn text(&mut self, s: &str) -> Result<(), Error> {
        if s.len() > 1024 {
            return Err(LIMIT.into());
        }
        self.u32(s.len() as u32)?;
        self.put(s.as_bytes())
    }
    fn available(&mut self, f: &available_70374::TableFacts) -> Result<(), Error> {
        self.u32(f.file_data_id())?;
        self.u64(f.declared_bytes)?;
        self.u64(f.available_bytes)?;
        self.bit(f.full_ckey_verified)?;
        self.bit(f.encoded_key_authenticated)?;
        for n in [
            f.declared_direct_rows,
            f.declared_copy_records,
            f.direct_rows,
            f.copy_records,
            f.materialized_rows,
        ] {
            self.n(n)?;
        }
        self.n(f.sections().count())?;
        for s in f.sections() {
            self.u32(s.ordinal)?;
            self.u32(s.declared_records)?;
            self.u32(s.declared_copies)?;
            self.bit(s.state == forever_db2::availability::SectionState::Admitted)?;
        }
        Ok(())
    }
    pub(crate) fn capture(
        &mut self,
        context: Context,
        selected: Option<u32>,
        v: coverage::View<'_>,
    ) -> Result<(), Error> {
        let profile = match context.build {
            70425 => compatible::world::inventory::COVERAGE_PROFILE_GNU15_70425,
            70450 => compatible::world::inventory::COVERAGE_PROFILE_GNU15_70450,
            _ => return Err(SCHEMA.into()),
        };
        if !self.bytes.is_empty()
            || context.components != Component::Collision.bit()
            || v.profile() != profile
            || v.maps().len() > MAX_MAPS
        {
            return Err(SCHEMA.into());
        }
        self.put(MAGIC)?;
        self.text(&context.json())?;
        self.opt(selected)?;
        self.text(profile)?;
        self.available(v.map_table())?;
        self.available(v.startup().table())?;
        for f in v.liquids() {
            self.u32(f.source().file_data_id())?;
            self.n(f.source().byte_count())?;
            self.put(&f.source().first_magic().ok_or(SCHEMA)?)?;
            for n in [f.direct_rows, f.copy_records, f.materialized_rows] {
                self.n(n)?;
            }
        }
        self.n(v.startup().rows().len())?;
        for r in v.startup().rows() {
            self.n(r.ordinal)?;
            self.u32(r.display_id)?;
            self.u32(r.file_data_id)?;
            let code = match r.disposition {
                startup::Disposition::Normal(o) => match o {
                    gameobjects::Outcome::ZeroFileDataId => 0,
                    gameobjects::Outcome::Texture => 1,
                    gameobjects::Outcome::Collision(_) => 2,
                    gameobjects::Outcome::M2NoCollision(_) => 3,
                    gameobjects::Outcome::WmoDoodadOnly(_) => 4,
                },
                startup::Disposition::Omitted(o) => match o {
                    compatible::Omission::MissingRoot => 5,
                    compatible::Omission::MissingRequiredGroup => 6,
                    compatible::Omission::NoUsableCollisionAfterOmission => 7,
                    compatible::Omission::CompleteNoCollision => 8,
                },
            };
            self.put(&[code])?;
        }
        self.n(v.topology().all().len())?;
        for m in v.topology().all() {
            self.u32(m.id())?;
            self.opt(m.parent_map_id())?;
            self.u32(m.wdt_file_data_id())?;
            self.bit(m.active())?;
            self.n(m.physical_ordinal())?;
            self.n(m.depth())?;
            self.bit(m.is_parent())?;
        }
        self.n(v.maps().len())?;
        for m in v.maps() {
            self.u32(m.id())?;
            self.u32(m.wdt_file_data_id())?;
            self.opt(m.parent_map_id())?;
            self.bit(m.wdt() == coverage::Wdt::Present)?;
            self.bit(m.global_records().is_some())?;
            self.n(m.global_records().unwrap_or(0))?;
            if m.wdt() == coverage::Wdt::Present {
                for x in 0..64 {
                    for y in 0..64 {
                        let t = m.tile(x, y).ok_or(SCHEMA)?;
                        let own = match t.own() {
                            coverage::Own::Unlisted => 0,
                            coverage::Own::Missing => 1,
                            coverage::Own::Empty => 2,
                            coverage::Own::Output => 3,
                        };
                        let replay = match t.replay() {
                            coverage::Replay::None => 0,
                            coverage::Replay::Empty => 1,
                            coverage::Replay::Output => 2,
                        };
                        if t.replay_parent() != if replay == 0 { None } else { m.parent_map_id() } {
                            return Err(SCHEMA.into());
                        }
                        self.put(&[own | (replay << 2)])?;
                    }
                }
            }
        }
        let r = v.report();
        for n in [
            r.maps as u64,
            r.tile_positions,
            r.object_adts,
            r.wdt_files as u64,
            r.missing_wdts as u64,
            r.missing_object_adts,
            r.own_tiles,
            r.replayed_tiles,
            r.empty_replays,
            r.cached_tiles,
            r.placement_records,
            r.placement_bytes,
            r.model_reads,
            r.model_source_bytes,
            r.placement_callbacks,
        ] {
            self.u64(n)?;
        }
        self.count_at = self.bytes.len();
        self.u64(0)
    }
    pub(crate) fn artifact(
        &mut self,
        path: &str,
        bytes: usize,
        sha: [u8; 32],
    ) -> Result<(), Error> {
        if self.count_at == 0 || self.files >= MAX_FILES - 1 || path == PATH {
            return Err(LIMIT.into());
        }
        self.text(path)?;
        self.n(bytes)?;
        self.put(&sha)?;
        self.files += 1;
        self.total = self.total.checked_add(bytes as u64).ok_or(LIMIT)?;
        Ok(())
    }
    pub(crate) fn finish(&mut self) -> Result<&[u8], Error> {
        if self.count_at == 0 {
            return Err(SCHEMA.into());
        }
        self.bytes[self.count_at..self.count_at + 8]
            .copy_from_slice(&(self.files as u64).to_le_bytes());
        Ok(&self.bytes)
    }
    pub(crate) fn totals(&self) -> (usize, u64) {
        (self.files + 1, self.total + self.bytes.len() as u64)
    }
}
pub(crate) fn proof_path(id: u32) -> String {
    format!("vmaps/coverage/{id:04}.placements")
}
pub(crate) fn proof_header(id: u32, len: usize) -> Result<[u8; 16], Error> {
    if len > forever_vmap::placement::MAX_BYTES {
        return Err(LIMIT.into());
    }
    let mut b = [0; 16];
    b[..8].copy_from_slice(PROOF_MAGIC);
    b[8..12].copy_from_slice(&id.to_le_bytes());
    b[12..].copy_from_slice(&(len as u32).to_le_bytes());
    Ok(b)
}
pub(crate) fn proof(bytes: &[u8], id: u32) -> Result<&[u8], Error> {
    if bytes.len() < 16
        || bytes[..8] != *PROOF_MAGIC
        || bytes[8..12] != id.to_le_bytes()
        || bytes.len()
            != 16 + u32::from_le_bytes(bytes[12..16].try_into().map_err(|_| SCHEMA)?) as usize
    {
        return Err(SCHEMA.into());
    }
    Ok(&bytes[16..])
}
#[derive(Clone, Copy)]
pub(crate) struct Reader<'a> {
    b: &'a [u8],
    at: usize,
}
impl<'a> Reader<'a> {
    pub(crate) fn take(&mut self, n: usize) -> Result<&'a [u8], Error> {
        let end = self.at.checked_add(n).ok_or(SCHEMA)?;
        let b = self.b.get(self.at..end).ok_or(SCHEMA)?;
        self.at = end;
        Ok(b)
    }
    fn u32(&mut self) -> Result<u32, Error> {
        Ok(u32::from_le_bytes(
            self.take(4)?.try_into().map_err(|_| SCHEMA)?,
        ))
    }
    fn u64(&mut self) -> Result<u64, Error> {
        Ok(u64::from_le_bytes(
            self.take(8)?.try_into().map_err(|_| SCHEMA)?,
        ))
    }
    fn n(&mut self, cap: usize) -> Result<usize, Error> {
        let n = usize::try_from(self.u64()?).map_err(|_| LIMIT)?;
        if n > cap {
            return Err(LIMIT.into());
        }
        Ok(n)
    }
    fn bit(&mut self) -> Result<bool, Error> {
        match self.take(1)?[0] {
            0 => Ok(false),
            1 => Ok(true),
            _ => Err(SCHEMA.into()),
        }
    }
    fn opt(&mut self) -> Result<Option<u32>, Error> {
        let bit = self.bit()?;
        let n = self.u32()?;
        if !bit && n != 0 {
            return Err(SCHEMA.into());
        }
        Ok(bit.then_some(n))
    }
    fn text(&mut self) -> Result<&'a str, Error> {
        let n = self.u32()? as usize;
        if n > 1024 {
            return Err(LIMIT.into());
        }
        std::str::from_utf8(self.take(n)?).map_err(|_| SCHEMA.into())
    }
    fn available(&mut self, expected: u32) -> Result<(usize, usize), Error> {
        if self.u32()? != expected {
            return Err(SCHEMA.into());
        }
        let declared = self.u64()?;
        if declared < 204 || declared > forever_casc::collision::world::MAX_DB2_BYTES as u64 {
            return Err(LIMIT.into());
        }
        let available = self.u64()?;
        let full = self.bit()?;
        let encoded = self.bit()?;
        if available > declared || (full && available != declared) || (!full && !encoded) {
            return Err(SCHEMA.into());
        }
        let dr = self.n(200000)?;
        let dc = self.n(200000)?;
        let ar = self.n(200000)?;
        let ac = self.n(200000)?;
        let mat = self.n(200000)?;
        if ar > dr || ac > dc || mat > ar + ac {
            return Err(SCHEMA.into());
        }
        let n = self.n(16)?;
        let mut unknown = false;
        let mut before = None;
        let mut admitted_rows = 0usize;
        let mut admitted_copies = 0usize;
        let mut declared_rows = 0usize;
        let mut declared_copies = 0usize;
        for _ in 0..n {
            let ord = self.u32()?;
            if before.is_some_and(|v| v >= ord) {
                return Err(SCHEMA.into());
            }
            before = Some(ord);
            let rows = self.u32()? as usize;
            let copies = self.u32()? as usize;
            declared_rows = declared_rows.checked_add(rows).ok_or(LIMIT)?;
            declared_copies = declared_copies.checked_add(copies).ok_or(LIMIT)?;
            let admitted = self.bit()?;
            unknown |= !admitted;
            if admitted {
                admitted_rows = admitted_rows.checked_add(rows).ok_or(LIMIT)?;
                admitted_copies = admitted_copies.checked_add(copies).ok_or(LIMIT)?;
            }
        }
        if (full && unknown)
            || admitted_rows != ar
            || admitted_copies != ac
            || declared_rows != dr
            || declared_copies != dc
        {
            return Err(SCHEMA.into());
        }
        Ok((ar, mat))
    }
}
#[derive(Clone, Copy)]
pub(crate) struct StartupRow {
    pub display_id: u32,
    pub fdid: u32,
    pub disposition: u8,
}
#[derive(Clone, Copy)]
pub(crate) struct StartupIter<'a> {
    r: Reader<'a>,
    left: usize,
}
impl Iterator for StartupIter<'_> {
    type Item = StartupRow;
    fn next(&mut self) -> Option<Self::Item> {
        if self.left == 0 {
            return None;
        }
        self.left -= 1;
        self.r.u64().ok()?;
        Some(StartupRow {
            display_id: self.r.u32().ok()?,
            fdid: self.r.u32().ok()?,
            disposition: self.r.take(1).ok()?[0],
        })
    }
}
#[derive(Clone, Copy)]
pub(crate) struct Map<'a> {
    pub id: u32,
    pub wdt: u32,
    pub parent: Option<u32>,
    pub present: bool,
    pub global: usize,
    pub cells: &'a [u8],
}
#[derive(Clone, Copy)]
pub(crate) struct MapIter<'a> {
    r: Reader<'a>,
    left: usize,
}
impl<'a> Iterator for MapIter<'a> {
    type Item = Map<'a>;
    fn next(&mut self) -> Option<Self::Item> {
        if self.left == 0 {
            return None;
        }
        self.left -= 1;
        read_map(&mut self.r).ok()
    }
}
fn read_map<'a>(r: &mut Reader<'a>) -> Result<Map<'a>, Error> {
    let id = r.u32()?;
    let wdt = r.u32()?;
    let parent = r.opt()?;
    let present = r.bit()?;
    let global_present = r.bit()?;
    let global = r.n(1000000)?;
    if global_present != present || (!present && global != 0) {
        return Err(SCHEMA.into());
    }
    let cells = r.take(if present { 4096 } else { 0 })?;
    for &b in cells {
        if b > 9 || (b & 3) > 1 && b >> 2 != 0 || b >> 2 != 0 && parent.is_none() {
            return Err(SCHEMA.into());
        }
    }
    Ok(Map {
        id,
        wdt,
        parent,
        present,
        global,
        cells,
    })
}
#[derive(Clone, Copy)]
pub(crate) struct Inventory<'a> {
    r: Reader<'a>,
    left: usize,
}
pub(crate) struct Entry<'a> {
    pub path: &'a str,
    pub bytes: u64,
    pub sha: [u8; 32],
}
impl<'a> Iterator for Inventory<'a> {
    type Item = Entry<'a>;
    fn next(&mut self) -> Option<Self::Item> {
        if self.left == 0 {
            return None;
        }
        self.left -= 1;
        let path = self.r.text().ok()?;
        let bytes = self.r.u64().ok()?;
        let sha = self.r.take(32).ok()?.try_into().ok()?;
        Some(Entry { path, bytes, sha })
    }
}
#[derive(Clone, Copy)]
struct Topology {
    id: u32,
    parent: Option<u32>,
    wdt: u32,
    active: bool,
    ordinal: usize,
    depth: usize,
    is_parent: bool,
}
pub(crate) struct Coverage<'a> {
    pub context: Context,
    pub selected: Option<u32>,
    rows: StartupIter<'a>,
    records: u64,
    maps: MapIter<'a>,
    inventory: Inventory<'a>,
    topology: Vec<Topology>,
}
impl<'a> Coverage<'a> {
    pub(crate) fn parse(b: &'a [u8]) -> Result<Self, Error> {
        if b.len() > MAX_BYTES {
            return Err(LIMIT.into());
        }
        let mut r = Reader { b, at: 0 };
        if r.take(8)? != MAGIC {
            return Err(SCHEMA.into());
        }
        let context = Context::parse(r.text()?)?;
        if ![70425, 70450].contains(&context.build)
            || context.components != Component::Collision.bit()
        {
            return Err(SCHEMA.into());
        }
        let selected = r.opt()?;
        let expected_profile = match context.build {
            70425 => compatible::world::inventory::COVERAGE_PROFILE_GNU15_70425,
            70450 => compatible::world::inventory::COVERAGE_PROFILE_GNU15_70450,
            _ => return Err(SCHEMA.into()),
        };
        if r.text()? != expected_profile {
            return Err(SCHEMA.into());
        }
        let (map_direct, map_rows) =
            r.available(forever_casc::collision::world::MAP_FILE_DATA_ID)?;
        let (godi_rows, _) =
            r.available(forever_casc::collision::gameobjects::TABLE_FILE_DATA_ID)?;
        for expected in [
            forever_casc::collision::world::MATERIAL_FILE_DATA_ID,
            forever_casc::collision::world::TYPE_FILE_DATA_ID,
        ] {
            if r.u32()? != expected {
                return Err(SCHEMA.into());
            }
            let size = r.u64()?;
            if !(204..=forever_casc::collision::world::MAX_DB2_BYTES as u64).contains(&size) {
                return Err(LIMIT.into());
            }
            if r.take(4)? != b"WDC5" {
                return Err(SCHEMA.into());
            }
            let direct = r.n(200000)?;
            let copies = r.n(200000)?;
            let materialized = r.n(200000)?;
            if materialized < direct || materialized > direct.checked_add(copies).ok_or(LIMIT)? {
                return Err(SCHEMA.into());
            }
        }
        let n = r.n(gameobjects::MAX_DIRECT_ROWS)?;
        if n != godi_rows {
            return Err(SCHEMA.into());
        }
        let rows = StartupIter { r, left: n };
        for ordinal in 0..n {
            if r.n(gameobjects::MAX_DIRECT_ROWS)? != ordinal {
                return Err(SCHEMA.into());
            }
            r.u32()?;
            let fdid = r.u32()?;
            let disposition = r.take(1)?[0];
            if disposition > 8 || (disposition == 0) != (fdid == 0) {
                return Err(SCHEMA.into());
            }
        }
        let n = r.n(200000)?;
        if n != map_rows {
            return Err(SCHEMA.into());
        }
        let mut prev = None;
        let mut topology = Vec::new();
        topology
            .try_reserve_exact(n)
            .map_err(|_| "product_allocation")?;
        if topology.capacity() > n {
            return Err(LIMIT.into());
        }
        for expected_ordinal in 0..n {
            let id = r.u32()?;
            let parent = r.opt()?;
            let wdt = r.u32()?;
            let active = r.bit()?;
            let ordinal = r.n(200000)?;
            if ordinal != expected_ordinal
                || prev.is_some_and(|p| p >= ordinal)
                || active != (wdt != 0)
            {
                return Err(SCHEMA.into());
            }
            prev = Some(ordinal);
            let depth = r.n(forever_casc::collision::plan::MAX_DEPTH)?;
            let is_parent = r.bit()?;
            topology.push(Topology {
                id,
                parent,
                wdt,
                active,
                ordinal,
                depth,
                is_parent,
            });
        }
        topology.sort_unstable_by_key(|t| t.id);
        if topology.windows(2).any(|p| p[0].id == p[1].id) {
            return Err(SCHEMA.into());
        }
        let mut parent_flags = Vec::new();
        parent_flags
            .try_reserve_exact(n)
            .map_err(|_| "product_allocation")?;
        if parent_flags.capacity() > n {
            return Err(LIMIT.into());
        }
        parent_flags.resize(n, 0u8);
        for t in &topology {
            if let Some(id) = t.parent {
                let at = topology
                    .binary_search_by_key(&id, |t| t.id)
                    .map_err(|_| SCHEMA)?;
                let p = topology[at];
                if p.ordinal >= map_direct || p.depth.checked_add(1) != Some(t.depth) {
                    return Err(SCHEMA.into());
                }
                parent_flags[at] = 1;
            } else if t.depth != 0 {
                return Err(SCHEMA.into());
            }
        }
        if topology
            .iter()
            .zip(&parent_flags)
            .any(|(t, b)| t.is_parent != (*b == 1))
        {
            return Err(SCHEMA.into());
        }

        let count = r.n(MAX_MAPS)?;
        let maps = MapIter { r, left: count };
        let mut prev = None;
        for _ in 0..count {
            let m = read_map(&mut r)?;
            if prev.is_some_and(|p| p >= m.id) {
                return Err(SCHEMA.into());
            }
            prev = Some(m.id);
            let t = topology
                .binary_search_by_key(&m.id, |t| t.id)
                .ok()
                .map(|i| topology[i])
                .ok_or(SCHEMA)?;
            if !t.active || t.wdt != m.wdt || t.parent != m.parent {
                return Err(SCHEMA.into());
            }
        }
        let mut selected_ids = Vec::new();
        selected_ids
            .try_reserve_exact(MAX_MAPS)
            .map_err(|_| "product_allocation")?;
        if selected_ids.capacity() > MAX_MAPS {
            return Err(LIMIT.into());
        }
        if let Some(id) = selected {
            let mut next = Some(id);
            let mut steps = 0;
            while let Some(id) = next {
                steps += 1;
                if steps > MAX_MAPS {
                    return Err(SCHEMA.into());
                }
                let t = topology
                    .binary_search_by_key(&id, |t| t.id)
                    .ok()
                    .map(|i| topology[i])
                    .ok_or(SCHEMA)?;
                if !t.active
                    || t.ordinal >= map_direct
                    || steps > forever_casc::collision::plan::MAX_DOMAIN_MAPS
                {
                    return Err(SCHEMA.into());
                }
                if selected_ids.len() == MAX_MAPS {
                    return Err(LIMIT.into());
                }
                selected_ids.push(id);
                next = t.parent;
            }
        } else {
            for t in &topology {
                if t.active {
                    if selected_ids.len() == MAX_MAPS {
                        return Err(LIMIT.into());
                    }
                    selected_ids.push(t.id);
                }
            }
        }
        selected_ids.sort_unstable();
        if selected_ids.windows(2).any(|p| p[0] == p[1])
            || selected_ids.iter().copied().ne(maps.map(|m| m.id))
        {
            return Err(SCHEMA.into());
        }
        let mut report = [0u64; 15];
        for v in &mut report {
            *v = r.u64()?;
        }
        let resource = forever_casc::collision::resources::Profile::DiskWorldV1;
        let stage_limits = resource.stage_limits();
        let collector_limits = resource.collector_limits();
        if report[10] > stage_limits.max_records as u64
            || report[14] > stage_limits.max_callbacks as u64
            || report[12] > collector_limits.max_reads as u64
            || report[13] > collector_limits.max_source_bytes as u64
            || report[11]
                > report[10]
                    .checked_mul((placement_max_record()) as u64)
                    .ok_or(LIMIT)?
        {
            return Err(LIMIT.into());
        }

        let mut present = 0u64;
        let mut missing_adt = 0u64;
        let mut own = 0u64;
        let mut own_output = 0u64;
        let mut replays = 0u64;
        let mut empty_replays = 0u64;
        let mut cached = 0u64;
        let mut global_records = 0u64;
        for m in maps {
            if !m.present {
                continue;
            }
            present = present.checked_add(1).ok_or(LIMIT)?;
            global_records = global_records.checked_add(m.global as u64).ok_or(LIMIT)?;
            let t = topology[topology
                .binary_search_by_key(&m.id, |t| t.id)
                .map_err(|_| SCHEMA)?];
            let parent = m
                .parent
                .and_then(|id| maps.into_iter().find(|p| p.id == id));
            for (cell, &b) in m.cells.iter().enumerate() {
                let o = b & 3;
                let replay = b >> 2;
                if o == 1 {
                    missing_adt = missing_adt.checked_add(1).ok_or(LIMIT)?;
                }
                if o >= 2 {
                    own = own.checked_add(1).ok_or(LIMIT)?;
                    if o == 3 {
                        own_output = own_output.checked_add(1).ok_or(LIMIT)?;
                    }
                    if t.is_parent {
                        cached = cached.checked_add(1).ok_or(LIMIT)?;
                    }
                }
                if replay != 0 {
                    replays = replays.checked_add(1).ok_or(LIMIT)?;
                    if replay == 1 {
                        empty_replays = empty_replays.checked_add(1).ok_or(LIMIT)?;
                    }
                }
                if o <= 1 {
                    let expected = if let Some(p) = parent.filter(|p| p.present) {
                        match p.cells[cell] & 3 {
                            2 => 1,
                            3 => 2,
                            _ => 0,
                        }
                    } else {
                        0
                    };
                    if replay != expected {
                        return Err(SCHEMA.into());
                    }
                }
            }
        }
        let lower_records = global_records
            .checked_add(own_output)
            .and_then(|n| n.checked_add(replays.checked_sub(empty_replays)?))
            .ok_or(LIMIT)?;
        let callbacks = present
            .checked_add(own)
            .and_then(|n| n.checked_add(replays))
            .ok_or(LIMIT)?;
        if report[0] != count as u64
            || report[1] != (count as u64).checked_mul(4096).ok_or(LIMIT)?
            || report[2] != own
            || report[3] != present
            || report[4] != (count as u64).checked_sub(present).ok_or(LIMIT)?
            || report[3].checked_add(report[4]) != Some(report[0])
            || report[5] != missing_adt
            || report[6] != own
            || report[7] != replays
            || report[8] != empty_replays
            || report[9] != cached
            || report[10] < lower_records
            || report[11] < report[10].checked_mul(39).ok_or(LIMIT)?
            || report[13] < report[12].checked_mul(4).ok_or(LIMIT)?
            || report[14] != callbacks
        {
            return Err(SCHEMA.into());
        }
        let count = r.n(MAX_FILES - 1)?;
        let inventory = Inventory { r, left: count };
        for _ in 0..count {
            let p = r.text()?;
            if p == PATH
                || !p.starts_with("vmaps/")
                || p.len() > 1024
                || p.split('/').any(|s| s.is_empty() || s == "." || s == "..")
            {
                return Err(SCHEMA.into());
            }
            if r.u64()? > MAX_BYTES as u64 {
                return Err(LIMIT.into());
            }
            r.take(32)?;
        }
        if r.at != b.len() {
            return Err(SCHEMA.into());
        }
        Ok(Self {
            context,
            selected,
            rows,
            records: report[10],
            maps,
            inventory,
            topology,
        })
    }
    pub(crate) fn records(&self) -> u64 {
        self.records
    }
    pub(crate) fn startup(&self) -> StartupIter<'a> {
        self.rows
    }
    pub(crate) fn parent(&self, id: u32) -> Result<Option<u32>, Error> {
        self.topology
            .binary_search_by_key(&id, |t| t.id)
            .ok()
            .map(|i| self.topology[i].parent)
            .ok_or_else(|| SCHEMA.into())
    }
    pub(crate) fn resident_bytes(&self) -> usize {
        self.topology.capacity() * size_of::<Topology>()
    }
    pub(crate) fn maps(&self) -> MapIter<'a> {
        self.maps
    }
    pub(crate) fn inventory(&self) -> Inventory<'a> {
        self.inventory
    }
    pub(crate) fn matches(&self, artifacts: &[Artifact]) -> Result<(), Error> {
        if artifacts.len() != self.inventory.left + 1 {
            return Err(SCHEMA.into());
        }
        for e in self.inventory() {
            let a = artifacts
                .binary_search_by(|a| {
                    a.path
                        .bytes()
                        .map(|c| c.to_ascii_lowercase())
                        .cmp(e.path.bytes().map(|c| c.to_ascii_lowercase()))
                })
                .ok()
                .map(|i| &artifacts[i])
                .ok_or(SCHEMA)?;
            if a.path != e.path
                || a.component != Component::Collision
                || a.bytes != e.bytes
                || a.sha256 != e.sha
            {
                return Err(SCHEMA.into());
            }
        }
        let mut paths = Vec::new();
        paths
            .try_reserve_exact(self.inventory.left)
            .map_err(|_| "product_allocation")?;
        if paths.capacity() > self.inventory.left {
            return Err(LIMIT.into());
        }
        paths.extend(self.inventory().map(|e| e.path));
        paths.sort_unstable();
        if paths.windows(2).any(|p| p[0] == p[1]) {
            return Err(SCHEMA.into());
        }
        Ok(())
    }
}
pub(crate) fn proof_digest(id: u32, bytes: &[u8]) -> Result<[u8; 32], Error> {
    use std::io::Read;
    let header = proof_header(id, bytes.len())?;
    integrity::reader(header.as_slice().chain(bytes), (16 + bytes.len()) as u64)
        .map_err(|_| SCHEMA.into())
}
#[cfg(test)]
pub(crate) fn fixture(context: Context, maps: &[(u32, Option<u32>, bool, u8)]) -> Encoder {
    fixture_rows(context, maps, &[])
}
#[cfg(test)]
fn fixture_rows(
    context: Context,
    maps: &[(u32, Option<u32>, bool, u8)],
    rows: &[(usize, u32, u32, u8)],
) -> Encoder {
    // Synthetic bytes exercise the offline hostile-input codec only. They do
    // not construct a genuine coverage View, SourceFacts or completed owner.
    let mut e = Encoder {
        bytes: Vec::with_capacity(65536),
        count_at: 0,
        files: 0,
        total: 0,
    };
    e.put(MAGIC).unwrap();
    e.text(&context.json()).unwrap();
    e.opt(None).unwrap();
    let profile = match context.build {
        70425 => compatible::world::inventory::COVERAGE_PROFILE_GNU15_70425,
        70450 => compatible::world::inventory::COVERAGE_PROFILE_GNU15_70450,
        _ => panic!("unsupported synthetic coverage build"),
    };
    e.text(profile).unwrap();
    for fdid in [
        forever_casc::collision::world::MAP_FILE_DATA_ID,
        gameobjects::TABLE_FILE_DATA_ID,
    ] {
        let count = if fdid == forever_casc::collision::world::MAP_FILE_DATA_ID {
            maps.len()
        } else {
            rows.len()
        };
        e.u32(fdid).unwrap();
        e.u64(204).unwrap();
        e.u64(204).unwrap();
        e.bit(true).unwrap();
        e.bit(false).unwrap();
        for v in [count, 0, count, 0, count] {
            e.n(v).unwrap();
        }
        e.n(1).unwrap();
        e.u32(0).unwrap();
        e.u32(count as u32).unwrap();
        e.u32(0).unwrap();
        e.bit(true).unwrap();
    }
    for fdid in [
        forever_casc::collision::world::MATERIAL_FILE_DATA_ID,
        forever_casc::collision::world::TYPE_FILE_DATA_ID,
    ] {
        e.u32(fdid).unwrap();
        e.u64(204).unwrap();
        e.put(b"WDC5").unwrap();
        for _ in 0..3 {
            e.n(0).unwrap();
        }
    }
    e.n(rows.len()).unwrap();
    for &(ordinal, id, fdid, code) in rows {
        e.n(ordinal).unwrap();
        e.u32(id).unwrap();
        e.u32(fdid).unwrap();
        e.put(&[code]).unwrap();
    }
    e.n(maps.len()).unwrap();
    for (ordinal, &(id, parent, _, _)) in maps.iter().enumerate() {
        e.u32(id).unwrap();
        e.opt(parent).unwrap();
        e.u32(100 + ordinal as u32).unwrap();
        e.bit(true).unwrap();
        e.n(ordinal).unwrap();
        let mut depth = 0;
        let mut next = parent;
        while let Some(p) = next {
            depth += 1;
            next = maps.iter().find(|m| m.0 == p).and_then(|m| m.1);
            if depth > 1024 {
                break;
            }
        }
        e.n(depth).unwrap();
        e.bit(maps.iter().any(|m| m.1 == Some(id))).unwrap();
    }
    e.n(maps.len()).unwrap();
    for (ordinal, &(id, parent, present, cell)) in maps.iter().enumerate() {
        e.u32(id).unwrap();
        e.u32(100 + ordinal as u32).unwrap();
        e.opt(parent).unwrap();
        e.bit(present).unwrap();
        e.bit(present).unwrap();
        e.n(0).unwrap();
        if present {
            for _ in 0..4096 {
                e.put(&[cell]).unwrap();
            }
        }
    }
    let present = maps.iter().filter(|m| m.2).count() as u64;
    let own = maps.iter().filter(|m| m.2 && m.3 & 3 >= 2).count() as u64 * 4096;
    let missing = maps.iter().filter(|m| m.2 && m.3 & 3 == 1).count() as u64 * 4096;
    let output = maps.iter().filter(|m| m.2 && m.3 & 3 == 3).count() as u64 * 4096;
    let replays = maps.iter().filter(|m| m.2 && m.3 >> 2 != 0).count() as u64 * 4096;
    let empty = maps.iter().filter(|m| m.2 && m.3 >> 2 == 1).count() as u64 * 4096;
    let cached = maps
        .iter()
        .filter(|m| m.2 && m.3 & 3 >= 2 && maps.iter().any(|t| t.1 == Some(m.0)))
        .count() as u64
        * 4096;
    let records = output + replays - empty;
    for n in [
        maps.len() as u64,
        (maps.len() as u64) * 4096,
        own,
        present,
        maps.len() as u64 - present,
        missing,
        own,
        replays,
        empty,
        cached,
        records,
        records * 39,
        0,
        0,
        present + own + replays,
    ] {
        e.u64(n).unwrap();
    }
    e.count_at = e.bytes.len();
    e.u64(0).unwrap();
    e
}
#[cfg(test)]
mod tests {
    use super::*;
    fn context() -> Context {
        Context {
            build: 70425,
            locale: forever_casc::Locale::EsEs,
            components: Component::Collision.bit(),
            source_sha256: [1; 32],
            plan_sha256: [2; 32],
            producer_sha256: [3; 32],
        }
    }
    #[test]
    fn coverage_v1_empty_and_missing_are_distinct() {
        let mut e = fixture(context(), &[(0, None, false, 0), (1, None, true, 2)]);
        let c = Coverage::parse(e.finish().unwrap()).unwrap();
        let m = c.maps().collect::<Vec<_>>();
        assert!(!m[0].present);
        assert!(m[0].cells.is_empty());
        assert!(m[1].present);
        assert_eq!(m[1].cells, [2; 4096]);
    }
    #[test]
    fn coverage_v1_replay_requires_parent_and_empty_is_barrier() {
        for cell in [4, 8, 5, 9] {
            let parent = if cell >> 2 == 1 { 2 } else { 3 };
            let mut e = fixture(
                context(),
                &[(0, None, true, parent), (1, Some(0), true, cell)],
            );
            assert!(Coverage::parse(e.finish().unwrap()).is_ok());
        }
        for cell in [4, 8, 6, 7, 10, 11, 255] {
            let mut e = fixture(context(), &[(1, None, true, cell)]);
            assert!(Coverage::parse(e.finish().unwrap()).is_err());
        }
    }
    #[test]
    fn coverage_v1_complete_framing_and_context_are_required() {
        let mut e = fixture(context(), &[]);
        let b = e.finish().unwrap().to_vec();
        for n in 0..b.len() {
            assert!(Coverage::parse(&b[..n]).is_err());
        }
        let mut x = b.clone();
        x.push(0);
        assert!(Coverage::parse(&x).is_err());
        let mut x = b;
        x[0] ^= 1;
        assert!(Coverage::parse(&x).is_err());
    }
    #[test]
    fn coverage_v1_70450_build_and_profile_are_independent() {
        let context = Context {
            build: 70450,
            ..context()
        };
        let mut e = fixture(context, &[]);
        let bytes = e.finish().unwrap().to_vec();
        let coverage = Coverage::parse(&bytes).unwrap();
        assert_eq!(coverage.context.build, 70450);

        let mut wrong_profile = bytes.clone();
        let expected = compatible::world::inventory::COVERAGE_PROFILE_GNU15_70450;
        let replacement = compatible::world::inventory::COVERAGE_PROFILE_GNU15_70425;
        let at = wrong_profile
            .windows(expected.len())
            .position(|window| window == expected.as_bytes())
            .unwrap();
        wrong_profile[at..at + expected.len()].copy_from_slice(replacement.as_bytes());
        assert!(Coverage::parse(&wrong_profile).is_err());

        let mut wrong_build = bytes;
        let build_field = b"\"build\":";
        let at = wrong_build
            .windows(build_field.len() + 5)
            .position(|window| window.starts_with(build_field) && window.ends_with(b"70450"))
            .unwrap()
            + build_field.len();
        wrong_build[at..at + 5].copy_from_slice(b"70374");
        assert!(Coverage::parse(&wrong_build).is_err());
    }
    #[test]
    fn coverage_v1_duplicate_or_unordered_map_ids_reject() {
        for maps in [
            &[(0, None, true, 0), (0, None, true, 0)][..],
            &[(1, None, true, 0), (0, None, true, 0)][..],
        ] {
            let mut e = fixture(context(), maps);
            assert!(Coverage::parse(e.finish().unwrap()).is_err());
        }
    }
    #[test]
    fn coverage_v1_inventory_digest_size_and_duplicates_are_closed() {
        let mut e = fixture(context(), &[]);
        e.artifact("vmaps/GameObjectModels.dtree", 8, [1; 32])
            .unwrap();
        let c = Coverage::parse(e.finish().unwrap()).unwrap();
        let mut a = vec![
            Artifact {
                component: Component::Collision,
                path: "vmaps/GameObjectModels.dtree".into(),
                bytes: 8,
                sha256: [1; 32],
            },
            Artifact {
                component: Component::Collision,
                path: PATH.into(),
                bytes: 0,
                sha256: [0; 32],
            },
        ];
        a.sort_by(|a, b| {
            a.path
                .bytes()
                .map(|c| c.to_ascii_lowercase())
                .cmp(b.path.bytes().map(|c| c.to_ascii_lowercase()))
        });
        c.matches(&a).unwrap();
        let at = a
            .iter()
            .position(|artifact| artifact.path == "vmaps/GameObjectModels.dtree")
            .unwrap();
        a[at].bytes += 1;
        assert!(c.matches(&a).is_err());
    }
    #[test]
    fn coverage_v1_witness_is_versioned_map_bound_and_exact() {
        let h = proof_header(7, 0).unwrap();
        assert_eq!(proof(&h, 7).unwrap(), []);
        assert!(proof(&h, 8).is_err());
        let mut extra = h.to_vec();
        extra.push(0);
        assert!(proof(&extra, 7).is_err());
        assert!(proof_header(0, placement_max() + 1).is_err());
    }
    fn placement_max() -> usize {
        forever_vmap::placement::MAX_BYTES
    }
    #[test]
    fn coverage_v1_hostile_report_scalars_cannot_overflow_or_wrap() {
        let mut e = fixture(context(), &[]);
        let at = e.count_at - 15 * 8;
        let original = e.finish().unwrap().to_vec();
        for i in 0..15 {
            let mut b = original.clone();
            b[at + i * 8..at + (i + 1) * 8].copy_from_slice(&u64::MAX.to_le_bytes());
            assert!(Coverage::parse(&b).is_err());
        }
    }
    #[test]
    fn coverage_v1_parent_cycles_and_absent_parents_are_not_unknown_success() {
        for maps in [
            &[(0, Some(99), true, 0)][..],
            &[(0, Some(1), true, 0), (1, Some(0), true, 0)][..],
        ] {
            let mut e = fixture(context(), maps);
            assert!(Coverage::parse(e.finish().unwrap()).is_err());
        }
    }
    #[test]
    fn coverage_v1_replay_cannot_ignore_immediate_own_empty_cache() {
        let mut e = fixture(context(), &[(0, None, true, 2), (1, Some(0), true, 0)]);
        assert!(Coverage::parse(e.finish().unwrap()).is_err());
        let mut e = fixture(context(), &[(0, None, true, 0), (1, Some(0), true, 8)]);
        assert!(Coverage::parse(e.finish().unwrap()).is_err());
    }

    #[test]
    fn coverage_v1_godi_ordinals_and_zero_fdid_dispositions_are_checked() {
        for rows in [&[(0, 1, 0, 0)][..], &[(0, 1, 5, 2), (1, 1, 6, 5)][..]] {
            let mut e = fixture_rows(context(), &[], rows);
            assert!(Coverage::parse(e.finish().unwrap()).is_ok());
        }
        for rows in [
            &[(1, 1, 0, 0)][..],
            &[(0, 1, 0, 2)][..],
            &[(0, 1, 5, 0)][..],
            &[(0, 1, 5, 9)][..],
            &[(0, 1, 5, 2), (0, 2, 6, 5)][..],
        ] {
            let mut e = fixture_rows(context(), &[], rows);
            assert!(Coverage::parse(e.finish().unwrap()).is_err());
        }
    }

    fn topology_offset(bytes: &[u8]) -> usize {
        let mut r = Reader { b: bytes, at: 0 };
        r.take(8).unwrap();
        r.text().unwrap();
        r.opt().unwrap();
        r.text().unwrap();
        r.available(forever_casc::collision::world::MAP_FILE_DATA_ID)
            .unwrap();
        r.available(gameobjects::TABLE_FILE_DATA_ID).unwrap();
        r.take(80).unwrap();
        let n = r.n(gameobjects::MAX_DIRECT_ROWS).unwrap();
        r.take(n * 17).unwrap();
        r.n(200000).unwrap();
        r.at
    }
    #[test]
    fn coverage_v1_topology_depth_parentflag_and_physical_ordinal_are_exact() {
        let mut e = fixture(context(), &[(0, None, true, 0)]);
        let original = e.finish().unwrap().to_vec();
        let at = topology_offset(&original);
        for (offset, b) in [
            (14, 1u64.to_le_bytes().to_vec()),
            (22, 1u64.to_le_bytes().to_vec()),
            (30, vec![1]),
        ] {
            let mut bytes = original.clone();
            bytes[at + offset..at + offset + b.len()].copy_from_slice(&b);
            assert!(Coverage::parse(&bytes).is_err());
        }
    }
}
