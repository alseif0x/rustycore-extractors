// SPDX-License-Identifier: GPL-3.0-or-later
// Authored 2026-10-15: explicit Linux70374 full available-map product.
//! Coverage is a producer claim bound to source/plan/producer identity. Offline
//! verification does not independently authenticate client Map/WDT selection.
use crate::{
    Error,
    plan::{Plan, Selection},
};
use forever_casc::{
    local_store::LocalStore,
    package::{self, Artifact, Component, Context, Run},
    terrain::{available_70374 as available, plan, run, tilelist},
};
use forever_db2::availability::{SectionCoverage, SectionState};
use std::fmt::Write;

pub const COVERAGE_PATH: &str = "maps/selected-terrain-coverage-v1.tsv";
pub const MAX_MAPS: usize = 4096;
pub const COVERAGE_BYTES: usize = 1024 * 1024;
const SINK_BYTES: usize = 8 * 1024 * 1024;
const OUTPUT_BYTES: u64 = 64 * 1024 * 1024 * 1024;
const WORDS: usize = 64;

pub fn package_limits() -> package::Limits {
    package::Limits {
        total_bytes: OUTPUT_BYTES,
        ..Default::default()
    }
}
fn require(ok: bool) -> Result<(), Error> {
    if ok {
        Ok(())
    } else {
        Err("product_terrain_coverage".into())
    }
}
fn number(s: &str) -> Result<u64, Error> {
    let n: u64 = s
        .parse()
        .map_err(|_| Error::from("product_terrain_coverage"))?;
    require(n.to_string() == s)?;
    Ok(n)
}
fn boolean(s: &str) -> Result<bool, Error> {
    match s {
        "true" => Ok(true),
        "false" => Ok(false),
        _ => Err("product_terrain_coverage".into()),
    }
}
fn fields<'a>(
    lines: &mut impl Iterator<Item = &'a str>,
    name: &str,
    count: usize,
) -> Result<Vec<&'a str>, Error> {
    let f: Vec<_> = lines
        .next()
        .ok_or("product_terrain_coverage")?
        .split('\t')
        .collect();
    require(f.len() == count && f[0] == name)?;
    Ok(f)
}
fn selection_text(selection: Selection) -> Result<String, Error> {
    match selection {
        Selection::All => Ok("all-available-maps".into()),
        Selection::Map(id) => Ok(format!("map:{id}")),
        _ => Err("product_terrain_selection".into()),
    }
}
fn list_path(id: u32) -> String {
    format!("maps/{id:04}.tilelist")
}
pub fn parse_list(path: &str) -> Result<u32, Error> {
    let n = path
        .strip_prefix("maps/")
        .and_then(|p| p.strip_suffix(".tilelist"))
        .ok_or("product_artifact_name")?;
    let id = n
        .parse()
        .map_err(|_| Error::from("product_artifact_name"))?;
    require(list_path(id) == path)?;
    Ok(id)
}
pub fn parse_tile(path: &str) -> Result<(u32, u8, u8), Error> {
    let n = path
        .strip_prefix("maps/")
        .and_then(|p| p.strip_suffix(".map"))
        .ok_or("product_artifact_name")?;
    let f: Vec<_> = n.split('_').collect();
    require(f.len() == 3)?;
    let id = f[0]
        .parse()
        .map_err(|_| Error::from("product_artifact_name"))?;
    let y = f[1]
        .parse()
        .map_err(|_| Error::from("product_artifact_name"))?;
    let x = f[2]
        .parse()
        .map_err(|_| Error::from("product_artifact_name"))?;
    require(
        plan::map_relative_path(id, x, y).map_err(|_| Error::from("product_artifact_name"))?
            == path,
    )?;
    Ok((id, x, y))
}

#[derive(Clone)]
pub struct Coverage {
    pub context: Context,
    pub selection: Selection,
    pub pack: bool,
    pub map: available::MapFacts,
    pub maps: Vec<(u32, u32)>,
}
impl Coverage {
    fn validate(&self) -> Result<(), Error> {
        self.context.validate()?;
        require(
            self.context.build == 70374 && self.context.components == Component::Terrain.bit(),
        )?;
        selection_text(self.selection)?;
        require(
            self.maps.len() <= MAX_MAPS
                && self.map.declared_bytes > 0
                && self.map.declared_bytes <= 16 * 1024 * 1024
                && self.map.available_bytes <= self.map.declared_bytes
                && (self.map.full_ckey_verified || self.map.encoded_key_authenticated),
        )?;
        let mut rows = 0u64;
        let mut copies = 0u64;
        let mut direct = 0u64;
        let mut admitted_copies = 0u64;
        let mut count = 0;
        let mut ended = false;
        for section in self.map.sections {
            match section {
                None => ended = true,
                Some(s) => {
                    require(!ended && s.ordinal as usize == count)?;
                    count += 1;
                    rows += u64::from(s.declared_records);
                    copies += u64::from(s.declared_copies);
                    if s.state == SectionState::Admitted {
                        direct += u64::from(s.declared_records);
                        admitted_copies += u64::from(s.declared_copies);
                    }
                }
            }
        }
        require(
            rows <= 100_000
                && copies <= 100_000
                && self.map.direct_rows as u64 == direct
                && self.map.copy_records as u64 == admitted_copies
                && self.map.materialized_rows as u64 <= direct + admitted_copies,
        )?;
        require(
            self.maps.len() <= self.map.materialized_rows
                && (!self.map.full_ckey_verified
                    || self.map.available_bytes == self.map.declared_bytes),
        )?;
        require(
            self.maps.windows(2).all(|p| p[0].0 < p[1].0) && self.maps.iter().all(|p| p.1 <= 4096),
        )?;
        match self.selection {
            Selection::Map(id) => require(self.maps.len() == 1 && self.maps[0].0 == id)?,
            Selection::All => require(self.maps.len() == self.map.materialized_rows)?,
            _ => return Err("product_terrain_selection".into()),
        }
        Ok(())
    }
    pub fn render(&self) -> Result<String, Error> {
        self.validate()?;
        let mut out = String::new();
        out.try_reserve_exact(COVERAGE_BYTES)
            .map_err(|_| Error::from("product_allocation"))?;
        write!(out,"terrain-coverage\t1\nprofile\t{}\ncontext\t{}\nselection\t{}\npack\t{}\nmap-bytes\t{}\t{}\nmap-auth\t{}\t{}\nmap-rows\t{}\t{}\t{}\n",available::FULL_MAP_PROFILE,self.context.json(),selection_text(self.selection)?,self.pack,self.map.declared_bytes,self.map.available_bytes,self.map.full_ckey_verified,self.map.encoded_key_authenticated,self.map.direct_rows,self.map.copy_records,self.map.materialized_rows).expect("String formatting");
        for s in self.map.sections.into_iter().flatten() {
            writeln!(
                out,
                "section\t{}\t{}\t{}\t{}",
                s.ordinal,
                s.declared_records,
                s.declared_copies,
                match s.state {
                    SectionState::Admitted => "admitted",
                    SectionState::UnknownTact => "unknown-tact",
                }
            )
            .expect("String formatting");
        }
        let present: u64 = self.maps.iter().map(|p| u64::from(p.1)).sum();
        for &(id, count) in &self.maps {
            writeln!(out, "map\t{id}\t{count}\t{count}\t{}", list_path(id))
                .expect("String formatting");
        }
        let coordinates = self.maps.len() as u64 * 4096;
        writeln!(
            out,
            "finish\t{}\t{coordinates}\t{present}\t{}\t{present}\t{}",
            self.maps.len(),
            coordinates - present,
            self.maps.len()
        )
        .expect("String formatting");
        require(out.len() <= COVERAGE_BYTES)?;
        Ok(out)
    }
    pub fn parse(data: &[u8]) -> Result<Self, Error> {
        require(data.len() <= COVERAGE_BYTES && data.is_ascii())?;
        let text =
            std::str::from_utf8(data).map_err(|_| Error::from("product_terrain_coverage"))?;
        let mut lines = text.lines();
        require(fields(&mut lines, "terrain-coverage", 2)?[1] == "1")?;
        require(fields(&mut lines, "profile", 2)?[1] == available::FULL_MAP_PROFILE)?;
        let context = Context::parse(fields(&mut lines, "context", 2)?[1])?;
        let selected = fields(&mut lines, "selection", 2)?;
        let selection = if selected[1] == "all-available-maps" {
            Selection::All
        } else {
            Selection::Map(
                u32::try_from(number(
                    selected[1]
                        .strip_prefix("map:")
                        .ok_or("product_terrain_coverage")?,
                )?)
                .map_err(|_| Error::from("product_terrain_coverage"))?,
            )
        };
        let pack = boolean(fields(&mut lines, "pack", 2)?[1])?;
        let f = fields(&mut lines, "map-bytes", 3)?;
        let declared_bytes = number(f[1])?;
        let available_bytes = number(f[2])?;
        let f = fields(&mut lines, "map-auth", 3)?;
        let full_ckey_verified = boolean(f[1])?;
        let encoded_key_authenticated = boolean(f[2])?;
        let f = fields(&mut lines, "map-rows", 4)?;
        let convert =
            |s| usize::try_from(number(s)?).map_err(|_| Error::from("product_terrain_coverage"));
        let direct_rows = convert(f[1])?;
        let copy_records = convert(f[2])?;
        let materialized_rows = convert(f[3])?;
        let mut sections = [None; 16];
        let mut count = 0;
        let mut maps = Vec::new();
        maps.try_reserve_exact(MAX_MAPS)
            .map_err(|_| Error::from("product_allocation"))?;
        let mut map_started = false;
        for line in lines {
            let f: Vec<_> = line.split('\t').collect();
            match f.first().copied() {
                Some("section") if !map_started && f.len() == 5 && count < 16 => {
                    let u = |s| {
                        u32::try_from(number(s)?)
                            .map_err(|_| Error::from("product_terrain_coverage"))
                    };
                    sections[count] = Some(SectionCoverage {
                        ordinal: u(f[1])?,
                        declared_records: u(f[2])?,
                        declared_copies: u(f[3])?,
                        state: match f[4] {
                            "admitted" => SectionState::Admitted,
                            "unknown-tact" => SectionState::UnknownTact,
                            _ => return Err("product_terrain_coverage".into()),
                        },
                    });
                    count += 1;
                }
                Some("map") if f.len() == 5 && maps.len() < MAX_MAPS => {
                    map_started = true;
                    let id = u32::try_from(number(f[1])?)
                        .map_err(|_| Error::from("product_terrain_coverage"))?;
                    let n = u32::try_from(number(f[2])?)
                        .map_err(|_| Error::from("product_terrain_coverage"))?;
                    require(number(f[3])? == u64::from(n) && f[4] == list_path(id))?;
                    maps.push((id, n));
                }
                Some("finish") => {
                    map_started = true;
                }
                _ => return Err("product_terrain_coverage".into()),
            }
        }
        let result = Self {
            context,
            selection,
            pack,
            map: available::MapFacts {
                declared_bytes,
                available_bytes,
                full_ckey_verified,
                encoded_key_authenticated,
                work: Default::default(),
                direct_rows,
                copy_records,
                materialized_rows,
                sections,
            },
            maps,
        };
        require(result.render()?.as_bytes() == data)?;
        Ok(result)
    }
}

struct Bits {
    id: u32,
    maps: [u64; WORDS],
    listed: [u64; WORDS],
    list: bool,
}
impl Bits {
    fn new(id: u32) -> Self {
        Self {
            id,
            maps: [0; WORDS],
            listed: [0; WORDS],
            list: false,
        }
    }
    fn count(&self) -> u32 {
        self.maps.iter().map(|w| w.count_ones()).sum()
    }
    fn tile(&mut self, x: u8, y: u8) -> Result<(), Error> {
        let n = usize::from(y) * 64 + usize::from(x);
        let bit = 1u64 << (n % 64);
        require(self.maps[n / 64] & bit == 0)?;
        self.maps[n / 64] |= bit;
        Ok(())
    }
    fn tile_list(&mut self, bytes: &[u8]) -> Result<(), Error> {
        tilelist::verify_70374(bytes, 70374).map_err(|_| Error::from("product_tilelist"))?;
        require(!self.list)?;
        self.list = true;
        for (i, &b) in bytes[12..].iter().enumerate() {
            if b == b'1' {
                let n = 4095 - i;
                self.listed[n / 64] |= 1u64 << (n % 64);
            }
        }
        Ok(())
    }
}
fn map_bits(maps: &mut Vec<Bits>, id: u32) -> Result<&mut Bits, Error> {
    let i = match maps.binary_search_by_key(&id, |p| p.id) {
        Ok(i) => i,
        Err(i) => {
            require(maps.len() < MAX_MAPS)?;
            maps.try_reserve_exact(1)
                .map_err(|_| Error::from("product_allocation"))?;
            maps.insert(i, Bits::new(id));
            i
        }
    };
    Ok(&mut maps[i])
}
struct Sink<'a> {
    run: &'a mut Run,
    maps: Vec<Bits>,
    error: Option<Error>,
}
impl Sink<'_> {
    fn put(&mut self, path: &str, bytes: &[u8]) -> Result<(), run::SinkError> {
        self.run
            .put(Component::Terrain, path, bytes)
            .map(|_| ())
            .map_err(|e| {
                self.error = Some(e.into());
                run::SinkError
            })
    }
}
impl available::Sink for Sink<'_> {
    fn source(&mut self, source: run::SourceFacts) -> Result<(), run::SinkError> {
        if let run::Step::Wdt { map } = source.step {
            map_bits(&mut self.maps, map).map_err(|e| {
                self.error = Some(e);
                run::SinkError
            })?;
        }
        Ok(())
    }
    fn tile(
        &mut self,
        context: forever_terrain::conversion::Context,
        _: run::SourceFacts,
        tile: &forever_terrain::conversion::ConvertedTile,
    ) -> Result<(), run::SinkError> {
        let path = plan::map_relative_path(context.map_id, context.tile_x, context.tile_y)
            .map_err(|_| run::SinkError)?;
        self.put(&path, tile.bytes())?;
        map_bits(&mut self.maps, context.map_id)
            .and_then(|m| m.tile(context.tile_x, context.tile_y))
            .map_err(|e| {
                self.error = Some(e);
                run::SinkError
            })
    }
    fn tile_list(&mut self, id: u32, list: &tilelist::TileList) -> Result<(), run::SinkError> {
        let m = map_bits(&mut self.maps, id).map_err(|_| run::SinkError)?;
        m.tile_list(list.bytes())
            .and_then(|()| require(m.maps == m.listed))
            .map_err(|e| {
                self.error = Some(e);
                run::SinkError
            })?;
        self.put(&list_path(id), list.bytes())
    }
}
// Current upstream ExtractMaps continues after failed ConvertADT/WDT loads.
// This product deliberately fails the whole known-present selection instead;
// missing-input failure parity with that native loop is not claimed.
pub fn extract(
    product: &Plan,
    store: &mut LocalStore,
    run: &mut Run,
    context: Context,
) -> Result<(), Error> {
    let mut limits = available::FullMapLimits::default();
    // Common package's maximum retained metadata plus fixed bounded sink/codec.
    let reservation = package_limits()
        .metadata_bytes
        .checked_add(SINK_BYTES)
        .ok_or("product_terrain_memory")?;
    limits.resources.max_memory_bytes = limits
        .resources
        .max_memory_bytes
        .checked_sub(reservation)
        .ok_or("product_terrain_memory")?;
    limits.resources.max_output_bytes = OUTPUT_BYTES - COVERAGE_BYTES as u64;
    let ids;
    let selection = match product.terrain {
        Selection::All => plan::Selection::AllMaps,
        Selection::Map(id) => {
            ids = [id];
            plan::Selection::MapIds(&ids)
        }
        _ => return Err("product_terrain_selection".into()),
    };
    let mut map_state = Vec::new();
    map_state
        .try_reserve_exact(MAX_MAPS)
        .map_err(|_| Error::from("product_allocation"))?;
    require(
        map_state
            .capacity()
            .checked_mul(std::mem::size_of::<Bits>())
            .and_then(|n| n.checked_add(2 * COVERAGE_BYTES))
            .is_some_and(|n| n <= SINK_BYTES),
    )?;
    let mut sink = Sink {
        run,
        maps: map_state,
        error: None,
    };
    let completed = available::run_full_maps(store, selection, product.pack, limits, &mut sink)
        .map_err(|failure| {
            sink.error
                .take()
                .unwrap_or_else(|| failure.error.code().into())
        })?;
    let plan = completed.plan();
    let stats = completed.stats();
    require(sink.maps.iter().all(|m| m.list && m.maps == m.listed))?;
    let mut maps = Vec::new();
    maps.try_reserve_exact(sink.maps.len())
        .map_err(|_| Error::from("product_allocation"))?;
    for m in &sink.maps {
        maps.push((m.id, m.count()));
    }
    let present: u64 = maps.iter().map(|p| u64::from(p.1)).sum();
    require(
        plan.maps == maps.len()
            && plan.selected_coordinates == maps.len() as u64 * 4096
            && plan.present_tiles == present
            && plan.absent_tiles == plan.selected_coordinates - present
            && stats.converted_tiles == present
            && stats.tile_lists == maps.len() as u64,
    )?;
    let coverage = Coverage {
        context,
        selection: product.terrain,
        pack: product.pack,
        map: completed.map(),
        maps,
    }
    .render()?;
    store
        .verify_unchanged()
        .map_err(|e| Error::from(e.code()))?;
    sink.run
        .put(Component::Terrain, COVERAGE_PATH, coverage.as_bytes())?;
    store
        .verify_unchanged()
        .map_err(|e| Error::from(e.code()))?;
    Ok(())
}

pub struct VerifyFacts {
    coverage: Option<Coverage>,
    maps: Vec<Bits>,
}
impl VerifyFacts {
    pub fn new() -> Self {
        Self {
            coverage: None,
            maps: Vec::new(),
        }
    }
    pub fn inspect(&mut self, artifact: &Artifact, bytes: &[u8]) -> Result<(), Error> {
        if artifact.path == COVERAGE_PATH {
            require(self.coverage.is_none())?;
            self.coverage = Some(Coverage::parse(bytes)?);
            return Ok(());
        }
        if artifact.path.ends_with(".tilelist") {
            let id = parse_list(&artifact.path)?;
            map_bits(&mut self.maps, id)?.tile_list(bytes)
        } else {
            let (id, x, y) = parse_tile(&artifact.path)?;
            forever_terrain::maps::verify_70374(bytes, 70374, forever_terrain::maps::MAX_MAP_BYTES)
                .map_err(|e| Error::from(e.code()))?;
            map_bits(&mut self.maps, id)?.tile(x, y)
        }
    }
    pub fn finish(self, product: &Plan, context: Context) -> Result<(), Error> {
        let coverage = self.coverage.ok_or("product_terrain_coverage")?;
        require(
            coverage.context == context
                && coverage.selection == product.terrain
                && coverage.pack == product.pack
                && self.maps.len() == coverage.maps.len(),
        )?;
        for (actual, &(id, count)) in self.maps.iter().zip(&coverage.maps) {
            require(
                actual.id == id
                    && actual.list
                    && actual.maps == actual.listed
                    && actual.count() == count,
            )?;
        }
        Ok(())
    }
}
pub fn admit_metadata(product: &Plan, artifacts: &[Artifact]) -> Result<(), Error> {
    let mut coverage = false;
    let mut lists = Vec::new();
    let mut tiles = Vec::new();
    let mut total = 0u64;
    lists
        .try_reserve_exact(MAX_MAPS)
        .map_err(|_| Error::from("product_allocation"))?;
    tiles
        .try_reserve_exact(MAX_MAPS)
        .map_err(|_| Error::from("product_allocation"))?;
    for a in artifacts {
        require(a.component == Component::Terrain && a.bytes > 0)?;
        total = total.checked_add(a.bytes).ok_or("product_file_limit")?;
        require(total <= OUTPUT_BYTES)?;
        if a.path == COVERAGE_PATH {
            require(!coverage && a.bytes <= COVERAGE_BYTES as u64)?;
            coverage = true;
        } else if a.path.ends_with(".tilelist") {
            require(a.bytes == tilelist::BYTES as u64 && lists.len() < MAX_MAPS)?;
            lists.push(parse_list(&a.path)?);
        } else {
            require(a.bytes <= forever_terrain::maps::MAX_MAP_BYTES as u64)?;
            let (id, _, _) = parse_tile(&a.path)?;
            if !tiles.contains(&id) {
                require(tiles.len() < MAX_MAPS)?;
                tiles.push(id);
            }
        }
    }
    lists.sort_unstable();
    require(
        coverage
            && lists.windows(2).all(|p| p[0] != p[1])
            && tiles.iter().all(|id| lists.binary_search(id).is_ok()),
    )?;
    if let Selection::Map(id) = product.terrain {
        require(lists == [id])?;
    }
    Ok(())
}

#[cfg(test)]
mod tests;
