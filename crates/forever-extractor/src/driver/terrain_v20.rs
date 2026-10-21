// SPDX-License-Identifier: GPL-3.0-or-later
// Authored 2026-10-19: Linux build-70450 compatible available-Map terrain product.
//! Producer-reported coverage only. Partially readable Maps retain separate
//! CKey and encoded-key facts; this envelope makes no whole-client coverage,
//! build-format parity, real-extraction or runtime claim.

use crate::{
    Error,
    plan::{Plan, Selection},
};
use forever_casc::{
    local_store::LocalStore,
    package::{self, Artifact, Component, Context, Run},
    terrain::{available_70450 as available, plan, run, tilelist},
};
use forever_db2::availability::{SectionCoverage, SectionState};
use std::fmt::Write;

pub const COVERAGE_PATH: &str = "maps/compatible-terrain-coverage-v2.tsv";
pub const COVERAGE_PATH_V24: &str = "maps/compatible-terrain-coverage-v3.tsv";
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

pub fn package_limits_v24() -> package::Limits {
    package_limits()
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
    let values: Vec<_> = lines
        .next()
        .ok_or("product_terrain_coverage")?
        .splitn(count + 1, '\t')
        .collect();
    require(values.len() == count && values[0] == name)?;
    Ok(values)
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
    let digits = path
        .strip_prefix("maps/")
        .and_then(|p| p.strip_suffix(".tilelist"))
        .ok_or("product_artifact_name")?;
    let id = digits
        .parse()
        .map_err(|_| Error::from("product_artifact_name"))?;
    require(list_path(id) == path)?;
    Ok(id)
}
pub fn parse_tile(path: &str) -> Result<(u32, u8, u8), Error> {
    let name = path
        .strip_prefix("maps/")
        .and_then(|p| p.strip_suffix(".map"))
        .ok_or("product_artifact_name")?;
    let fields: Vec<_> = name.split('_').collect();
    require(fields.len() == 3)?;
    let id = fields[0]
        .parse()
        .map_err(|_| Error::from("product_artifact_name"))?;
    let y: u8 = fields[1]
        .parse()
        .map_err(|_| Error::from("product_artifact_name"))?;
    let x: u8 = fields[2]
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
    pub maps: Vec<available::CompatibleMapOutcome>,
    /// Present only in v24 coverage; rows are in the selected producer order.
    pub map_wdt_file_data_ids: Option<Vec<(u32, u32)>>,
}
impl Coverage {
    fn validate(&self) -> Result<(), Error> {
        self.context.validate()?;
        let component_mask = if self.map_wdt_file_data_ids.is_some() {
            Component::Db2.bit() | Component::Terrain.bit()
        } else {
            Component::Terrain.bit()
        };
        require(self.context.build == 70450 && self.context.components == component_mask)?;
        selection_text(self.selection)?;
        require(
            self.maps.len() <= MAX_MAPS
                && self.map.declared_bytes > 0
                && self.map.declared_bytes <= 16 * 1024 * 1024
                && self.map.available_bytes <= self.map.declared_bytes
                && (self.map.full_ckey_verified || self.map.encoded_key_authenticated)
                && (!self.map.full_ckey_verified
                    || self.map.available_bytes == self.map.declared_bytes)
                && (self.map.available_bytes == self.map.declared_bytes
                    || (!self.map.full_ckey_verified && self.map.encoded_key_authenticated)),
        )?;
        let (mut declared_rows, mut declared_copies, mut admitted_rows, mut admitted_copies) =
            (0u64, 0u64, 0u64, 0u64);
        let mut count = 0usize;
        let mut ended = false;
        for section in self.map.sections {
            match section {
                None => ended = true,
                Some(section) => {
                    require(!ended && section.ordinal as usize == count)?;
                    count += 1;
                    declared_rows += u64::from(section.declared_records);
                    declared_copies += u64::from(section.declared_copies);
                    if section.state == SectionState::Admitted {
                        admitted_rows += u64::from(section.declared_records);
                        admitted_copies += u64::from(section.declared_copies);
                    }
                }
            }
        }
        require(
            declared_rows <= 100_000
                && declared_copies <= 100_000
                && self.map.direct_rows as u64 == admitted_rows
                && self.map.copy_records as u64 == admitted_copies
                && self.map.materialized_rows as u64 <= admitted_rows + admitted_copies
                && self.maps.len() <= self.map.materialized_rows,
        )?;
        for (index, outcome) in self.maps.iter().enumerate() {
            require(
                outcome.plan_index == index
                    && outcome.materialized_ordinal < self.map.materialized_rows
                    && !self.maps[..index]
                        .iter()
                        .any(|prior| prior.map_id == outcome.map_id)
                    && (index == 0
                        || self.maps[index - 1].materialized_ordinal
                            < outcome.materialized_ordinal),
            )?;
            match outcome.wdt {
                available::WdtDisposition::Loaded => {
                    let present = outcome
                        .known_present_tiles
                        .ok_or("product_terrain_coverage")?;
                    require(
                        present <= 4096
                            && outcome.converted_tiles <= present
                            && outcome.missing_root_adts == present - outcome.converted_tiles,
                    )?;
                }
                available::WdtDisposition::Unavailable => require(
                    outcome.known_present_tiles.is_none()
                        && outcome.converted_tiles == 0
                        && outcome.missing_root_adts == 0,
                )?,
            }
        }
        if let Some(identities) = &self.map_wdt_file_data_ids {
            require(identities.len() == self.maps.len())?;
            for (identity, outcome) in identities.iter().zip(&self.maps) {
                require(identity.0 == outcome.map_id && identity.1 != 0)?;
            }
        }
        match self.selection {
            Selection::Map(id) => {
                require(self.maps.len() == 1 && self.maps[0].map_id == id)?;
            }
            Selection::All => require(self.maps.len() == self.map.materialized_rows)?,
            _ => return Err("product_terrain_selection".into()),
        }
        Ok(())
    }

    pub fn render(&self) -> Result<String, Error> {
        self.validate()?;
        let mut output = String::new();
        output
            .try_reserve_exact(COVERAGE_BYTES)
            .map_err(|_| Error::from("product_allocation"))?;
        let version = if self.map_wdt_file_data_ids.is_some() {
            3
        } else {
            2
        };
        write!(
            output,
            "terrain-coverage\t{version}\nprofile\t{}\ncontext\t{}\nselection\t{}\npack\t{}\nmap-bytes\t{}\t{}\nmap-auth\t{}\t{}\nmap-rows\t{}\t{}\t{}\n",
            available::COMPATIBLE_FULL_MAP_PROFILE,
            self.context.json(),
            selection_text(self.selection)?,
            self.pack,
            self.map.declared_bytes,
            self.map.available_bytes,
            self.map.full_ckey_verified,
            self.map.encoded_key_authenticated,
            self.map.direct_rows,
            self.map.copy_records,
            self.map.materialized_rows,
        )
        .expect("String formatting");
        for section in self.map.sections.into_iter().flatten() {
            writeln!(
                output,
                "section\t{}\t{}\t{}\t{}",
                section.ordinal,
                section.declared_records,
                section.declared_copies,
                match section.state {
                    SectionState::Admitted => "admitted",
                    SectionState::UnknownTact => "unknown-tact",
                }
            )
            .expect("String formatting");
        }
        let mut known = 0u64;
        let mut converted = 0u64;
        let mut missing = 0u64;
        let mut unavailable = 0u64;
        for (index, outcome) in self.maps.iter().enumerate() {
            let (wdt, present) = match outcome.wdt {
                available::WdtDisposition::Loaded => {
                    ("loaded", outcome.known_present_tiles.unwrap().to_string())
                }
                available::WdtDisposition::Unavailable => {
                    unavailable += 1;
                    ("unavailable", "unknown".into())
                }
            };
            known += outcome.known_present_tiles.unwrap_or(0) as u64;
            converted += outcome.converted_tiles as u64;
            missing += outcome.missing_root_adts as u64;
            if let Some(identities) = &self.map_wdt_file_data_ids {
                writeln!(
                    output,
                    "map\t{}\t{}\t{}\t{}\t{wdt}\t{present}\t{}\t{}\t{}",
                    outcome.plan_index,
                    outcome.materialized_ordinal,
                    outcome.map_id,
                    identities[index].1,
                    outcome.converted_tiles,
                    outcome.missing_root_adts,
                    list_path(outcome.map_id),
                )
                .expect("String formatting");
            } else {
                writeln!(
                    output,
                    "map\t{}\t{}\t{}\t{wdt}\t{present}\t{}\t{}\t{}",
                    outcome.plan_index,
                    outcome.materialized_ordinal,
                    outcome.map_id,
                    outcome.converted_tiles,
                    outcome.missing_root_adts,
                    list_path(outcome.map_id),
                )
                .expect("String formatting");
            }
        }
        let maps = self.maps.len() as u64;
        let loaded = maps - unavailable;
        writeln!(
            output,
            "finish\t{maps}\t{}\t{loaded}\t{unavailable}\t{known}\t{}\t{}\t{converted}\t{missing}\t{maps}",
            maps * 4096,
            loaded * 4096 - known,
            unavailable * 4096,
        )
        .expect("String formatting");
        require(output.len() <= COVERAGE_BYTES)?;
        Ok(output)
    }

    pub fn parse(data: &[u8]) -> Result<Self, Error> {
        require(data.len() <= COVERAGE_BYTES && data.is_ascii())?;
        let text =
            std::str::from_utf8(data).map_err(|_| Error::from("product_terrain_coverage"))?;
        let mut lines = text.lines();
        let version = fields(&mut lines, "terrain-coverage", 2)?[1];
        require(version == "2" || version == "3")?;
        require(fields(&mut lines, "profile", 2)?[1] == available::COMPATIBLE_FULL_MAP_PROFILE)?;
        let context = Context::parse(fields(&mut lines, "context", 2)?[1])?;
        let selected = fields(&mut lines, "selection", 2)?[1];
        let selection = if selected == "all-available-maps" {
            Selection::All
        } else {
            Selection::Map(
                u32::try_from(number(
                    selected
                        .strip_prefix("map:")
                        .ok_or("product_terrain_coverage")?,
                )?)
                .map_err(|_| Error::from("product_terrain_coverage"))?,
            )
        };
        let pack = boolean(fields(&mut lines, "pack", 2)?[1])?;
        let bytes = fields(&mut lines, "map-bytes", 3)?;
        let declared_bytes = number(bytes[1])?;
        let available_bytes = number(bytes[2])?;
        let auth = fields(&mut lines, "map-auth", 3)?;
        let full_ckey_verified = boolean(auth[1])?;
        let encoded_key_authenticated = boolean(auth[2])?;
        let rows = fields(&mut lines, "map-rows", 4)?;
        let usize_field = |value| {
            usize::try_from(number(value)?).map_err(|_| Error::from("product_terrain_coverage"))
        };
        let direct_rows = usize_field(rows[1])?;
        let copy_records = usize_field(rows[2])?;
        let materialized_rows = usize_field(rows[3])?;
        let mut sections = [None; 16];
        let mut section_count = 0;
        let mut maps = Vec::new();
        maps.try_reserve_exact(MAX_MAPS)
            .map_err(|_| Error::from("product_allocation"))?;
        let mut map_wdt_file_data_ids = if version == "3" {
            let mut identities = Vec::new();
            identities
                .try_reserve_exact(MAX_MAPS)
                .map_err(|_| Error::from("product_allocation"))?;
            Some(identities)
        } else {
            None
        };
        let mut maps_started = false;
        for line in lines {
            let values: Vec<_> = line.splitn(11, '\t').collect();
            match values.first().copied() {
                Some("section") if !maps_started && values.len() == 5 && section_count < 16 => {
                    let field = |value| {
                        u32::try_from(number(value)?)
                            .map_err(|_| Error::from("product_terrain_coverage"))
                    };
                    sections[section_count] = Some(SectionCoverage {
                        ordinal: field(values[1])?,
                        declared_records: field(values[2])?,
                        declared_copies: field(values[3])?,
                        state: match values[4] {
                            "admitted" => SectionState::Admitted,
                            "unknown-tact" => SectionState::UnknownTact,
                            _ => return Err("product_terrain_coverage".into()),
                        },
                    });
                    section_count += 1;
                }
                Some("map")
                    if values.len() == if version == "3" { 10 } else { 9 }
                        && maps.len() < MAX_MAPS =>
                {
                    maps_started = true;
                    let id = u32::try_from(number(values[3])?)
                        .map_err(|_| Error::from("product_terrain_coverage"))?;
                    let shift = usize::from(version == "3");
                    if let Some(identities) = &mut map_wdt_file_data_ids {
                        identities.push((
                            id,
                            u32::try_from(number(values[4])?)
                                .map_err(|_| Error::from("product_terrain_coverage"))?,
                        ));
                    }
                    let wdt = match values[4 + shift] {
                        "loaded" => available::WdtDisposition::Loaded,
                        "unavailable" => available::WdtDisposition::Unavailable,
                        _ => return Err("product_terrain_coverage".into()),
                    };
                    let known_present_tiles = if values[5 + shift] == "unknown" {
                        None
                    } else {
                        Some(usize_field(values[5 + shift])?)
                    };
                    require(values[8 + shift] == list_path(id))?;
                    maps.push(available::CompatibleMapOutcome {
                        map_id: id,
                        plan_index: usize_field(values[1])?,
                        materialized_ordinal: usize_field(values[2])?,
                        wdt,
                        known_present_tiles,
                        converted_tiles: usize_field(values[6 + shift])?,
                        missing_root_adts: usize_field(values[7 + shift])?,
                    });
                }
                Some("finish") => maps_started = true,
                _ => return Err("product_terrain_coverage".into()),
            }
        }
        let coverage = Self {
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
            map_wdt_file_data_ids,
        };
        require(coverage.render()?.as_bytes() == data)?;
        Ok(coverage)
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
        self.maps.iter().map(|word| word.count_ones()).sum()
    }
    fn tile(&mut self, x: u8, y: u8) -> Result<(), Error> {
        let index = usize::from(y) * 64 + usize::from(x);
        let bit = 1u64 << (index % 64);
        require(self.maps[index / 64] & bit == 0)?;
        self.maps[index / 64] |= bit;
        Ok(())
    }
    fn tile_list(&mut self, bytes: &[u8]) -> Result<(), Error> {
        tilelist::verify_70450(bytes, 70450).map_err(|_| Error::from("product_tilelist"))?;
        require(!self.list)?;
        self.list = true;
        for (index, &byte) in bytes[12..].iter().enumerate() {
            if byte == b'1' {
                let bit = 4095 - index;
                self.listed[bit / 64] |= 1u64 << (bit % 64);
            }
        }
        Ok(())
    }
}
fn map_bits(maps: &mut Vec<Bits>, id: u32) -> Result<&mut Bits, Error> {
    let index = match maps.binary_search_by_key(&id, |entry| entry.id) {
        Ok(index) => index,
        Err(index) => {
            require(maps.len() < MAX_MAPS)?;
            maps.try_reserve_exact(1)
                .map_err(|_| Error::from("product_allocation"))?;
            maps.insert(index, Bits::new(id));
            index
        }
    };
    Ok(&mut maps[index])
}

struct Sink<'a> {
    run: &'a mut Run,
    maps: Vec<Bits>,
    outcomes: Vec<available::CompatibleMapOutcome>,
    expected_map_identities: Option<Vec<ProducerMapIdentity>>,
    error: Option<Error>,
}
impl Sink<'_> {
    fn put(&mut self, path: &str, bytes: &[u8]) -> Result<(), run::SinkError> {
        self.run
            .put(Component::Terrain, path, bytes)
            .map(|_| ())
            .map_err(|error| {
                self.error = Some(error.into());
                run::SinkError
            })
    }
}
impl available::Sink for Sink<'_> {
    fn source(&mut self, source: run::SourceFacts) -> Result<(), run::SinkError> {
        if let run::Step::Wdt { map } = source.step {
            if let Some(expected) = &self.expected_map_identities {
                let matches = expected
                    .iter()
                    .find(|identity| identity.map_id == map)
                    .is_some_and(|identity| identity.wdt_file_data_id == source.file_data_id);
                if !matches {
                    self.error = Some("product_terrain_coverage_authentication".into());
                    return Err(run::SinkError);
                }
            }
            map_bits(&mut self.maps, map).map_err(|error| {
                self.error = Some(error);
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
            .and_then(|map| map.tile(context.tile_x, context.tile_y))
            .map_err(|error| {
                self.error = Some(error);
                run::SinkError
            })
    }
    fn tile_list(&mut self, id: u32, list: &tilelist::TileList) -> Result<(), run::SinkError> {
        let map = map_bits(&mut self.maps, id).map_err(|error| {
            self.error = Some(error);
            run::SinkError
        })?;
        map.tile_list(list.bytes())
            .and_then(|()| require(map.maps == map.listed))
            .map_err(|error| {
                self.error = Some(error);
                run::SinkError
            })?;
        self.put(&list_path(id), list.bytes())
    }
    fn compatible_map_outcome(
        &mut self,
        outcome: available::CompatibleMapOutcome,
    ) -> Result<(), run::SinkError> {
        let result = (|| {
            require(self.outcomes.len() < MAX_MAPS && outcome.plan_index == self.outcomes.len())?;
            if let Some(expected) = &self.expected_map_identities {
                let identity = expected
                    .get(outcome.plan_index)
                    .ok_or("product_terrain_coverage_authentication")?;
                require(
                    identity.map_id == outcome.map_id
                        && identity.materialized_ordinal == outcome.materialized_ordinal,
                )
                .map_err(|_| Error::from("product_terrain_coverage_authentication"))?;
            }
            let map = map_bits(&mut self.maps, outcome.map_id)?;
            require(
                map.list
                    && map.maps == map.listed
                    && map.count() as usize == outcome.converted_tiles,
            )?;
            self.outcomes.push(outcome);
            Ok(())
        })();
        result.map_err(|error: Error| {
            self.error = Some(error);
            run::SinkError
        })
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct ProducerMapIdentity {
    materialized_ordinal: usize,
    map_id: u32,
    wdt_file_data_id: u32,
}

struct ProducerMapSelection {
    materialized_rows: usize,
    selected: Vec<ProducerMapIdentity>,
}

/// Read the same build-70450 available Map projection that the terrain runner
/// consumes, retaining only the selected ordered row identities for v24.
fn producer_map_identities(
    store: &mut LocalStore,
    selection: Selection,
) -> Result<ProducerMapSelection, Error> {
    let source = store
        .read_file_data_id_available_70450_with_limit(run::MAP_ID, 16 * 1024 * 1024)
        .map_err(|_| Error::from("product_terrain_coverage_authentication"))?;
    let available = forever_db2::availability::read_map_70450(&source)
        .map_err(|_| Error::from("product_terrain_coverage_authentication"))?;
    let materialized = available
        .materialize()
        .map_err(|_| Error::from("product_terrain_coverage_authentication"))?;
    let rows = materialized.rows();
    let materialized_rows = rows.len();
    let mut selected = Vec::new();
    match selection {
        Selection::All => {
            require(materialized_rows <= MAX_MAPS)
                .map_err(|_| Error::from("product_terrain_coverage_authentication"))?;
            selected
                .try_reserve_exact(materialized_rows)
                .map_err(|_| Error::from("product_allocation"))?;
            for (materialized_ordinal, row) in rows.iter().enumerate() {
                let wdt_file_data_id = u32::try_from(row.fields.wdt_file_data_id)
                    .ok()
                    .filter(|id| *id != 0)
                    .ok_or("product_terrain_coverage_authentication")?;
                selected.push(ProducerMapIdentity {
                    materialized_ordinal,
                    map_id: row.id,
                    wdt_file_data_id,
                });
            }
        }
        Selection::Map(map_id) => {
            let mut matches = rows.iter().enumerate().filter(|(_, row)| row.id == map_id);
            let (materialized_ordinal, row) = matches
                .next()
                .ok_or("product_terrain_coverage_authentication")?;
            require(matches.next().is_none())
                .map_err(|_| Error::from("product_terrain_coverage_authentication"))?;
            let wdt_file_data_id = u32::try_from(row.fields.wdt_file_data_id)
                .ok()
                .filter(|id| *id != 0)
                .ok_or("product_terrain_coverage_authentication")?;
            selected
                .try_reserve_exact(1)
                .map_err(|_| Error::from("product_allocation"))?;
            selected.push(ProducerMapIdentity {
                materialized_ordinal,
                map_id,
                wdt_file_data_id,
            });
        }
        _ => return Err("product_terrain_selection".into()),
    }
    store
        .verify_unchanged()
        .map_err(|_| Error::from("product_terrain_coverage_authentication"))?;
    Ok(ProducerMapSelection {
        materialized_rows,
        selected,
    })
}

// Only explicit MissingFile at WDT/root ADT is omitted by the compatible route.
pub fn extract(
    product: &Plan,
    store: &mut LocalStore,
    run: &mut Run,
    context: Context,
) -> Result<(), Error> {
    let producer_maps = if product.is_v24() {
        Some(producer_map_identities(store, product.terrain)?)
    } else {
        None
    };
    let mut limits = available::FullMapLimits::default();
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
    let map_ids;
    let selection = match product.terrain {
        Selection::All => plan::Selection::AllMaps,
        Selection::Map(id) => {
            map_ids = [id];
            plan::Selection::MapIds(&map_ids)
        }
        _ => return Err("product_terrain_selection".into()),
    };
    let mut maps = Vec::new();
    maps.try_reserve_exact(MAX_MAPS)
        .map_err(|_| Error::from("product_allocation"))?;
    let mut outcomes = Vec::new();
    outcomes
        .try_reserve_exact(MAX_MAPS)
        .map_err(|_| Error::from("product_allocation"))?;
    require(
        maps.capacity()
            .checked_mul(std::mem::size_of::<Bits>())
            .and_then(|bytes| {
                outcomes
                    .capacity()
                    .checked_mul(std::mem::size_of::<available::CompatibleMapOutcome>())
                    .and_then(|more| bytes.checked_add(more))
            })
            .and_then(|bytes| bytes.checked_add(2 * COVERAGE_BYTES))
            .is_some_and(|bytes| bytes <= SINK_BYTES),
    )?;
    let mut sink = Sink {
        run,
        maps,
        outcomes,
        expected_map_identities: producer_maps.as_ref().map(|maps| maps.selected.clone()),
        error: None,
    };
    let completed =
        available::run_compatible_full_maps(store, selection, product.pack, limits, &mut sink)
            .map_err(|failure| {
                sink.error
                    .take()
                    .unwrap_or_else(|| failure.error.code().into())
            })?;
    let plan = completed.plan();
    let stats = completed.stats();
    require(
        sink.maps
            .iter()
            .all(|map| map.list && map.maps == map.listed),
    )?;
    let summary = completed.outcomes();
    let loaded = sink
        .outcomes
        .iter()
        .filter(|map| map.wdt == available::WdtDisposition::Loaded)
        .count();
    let unavailable = sink.outcomes.len() - loaded;
    let present: u64 = sink
        .outcomes
        .iter()
        .map(|map| map.known_present_tiles.unwrap_or(0) as u64)
        .sum();
    let converted: u64 = sink
        .outcomes
        .iter()
        .map(|map| map.converted_tiles as u64)
        .sum();
    let missing: u64 = sink
        .outcomes
        .iter()
        .map(|map| map.missing_root_adts as u64)
        .sum();
    require(
        plan.maps == sink.outcomes.len()
            && sink.maps.len() == plan.maps
            && plan.selected_coordinates == plan.maps as u64 * 4096
            && plan.known_present_tiles == present
            && plan.known_absent_tiles == loaded as u64 * 4096 - present
            && plan.unknown_coordinates == unavailable as u64 * 4096
            && plan.unavailable_wdts == unavailable
            && summary.loaded_wdts == loaded
            && summary.unavailable_wdts == unavailable
            && summary.known_present_tiles == present
            && summary.converted_tiles == converted
            && summary.missing_root_adts == missing
            && converted + missing == present
            && stats.converted_tiles == converted
            && stats.tile_lists == plan.maps as u64,
    )?;
    let map = completed.map();
    if let Some(producer_maps) = &producer_maps {
        require(
            map.materialized_rows == producer_maps.materialized_rows
                && sink.outcomes.len() == producer_maps.selected.len(),
        )
        .map_err(|_| Error::from("product_terrain_coverage_authentication"))?;
    }
    let map_wdt_file_data_ids = producer_maps.map(|maps| {
        maps.selected
            .into_iter()
            .map(|identity| (identity.map_id, identity.wdt_file_data_id))
            .collect()
    });
    let coverage = Coverage {
        context,
        selection: product.terrain,
        pack: product.pack,
        map,
        maps: std::mem::take(&mut sink.outcomes),
        map_wdt_file_data_ids,
    }
    .render()?;
    store
        .verify_unchanged()
        .map_err(|error| Error::from(error.code()))?;
    let path = if product.is_v24() {
        COVERAGE_PATH_V24
    } else {
        COVERAGE_PATH
    };
    sink.run
        .put(Component::Terrain, path, coverage.as_bytes())?;
    store
        .verify_unchanged()
        .map_err(|error| Error::from(error.code()))?;
    Ok(())
}

pub fn extract_v24(
    product: &Plan,
    store: &mut LocalStore,
    run: &mut Run,
    context: Context,
) -> Result<(), Error> {
    if !product.is_v24() {
        return Err("product_terrain_profile".into());
    }
    extract(product, store, run, context)
}

/// Independently decoded Map identity retained from the sealed v24 DB2 artifact.
/// This is a source projection, not proof of whole-client Map completeness.
pub(crate) struct AuthenticatedMap {
    declared_bytes: u64,
    direct_rows: usize,
    copy_records: usize,
    materialized_rows: usize,
    sections: [Option<SectionCoverage>; 16],
    rows: Vec<(u32, u32)>,
}

struct SealedMapSource<'a>(&'a [u8]);
impl forever_db2::availability::Source for SealedMapSource<'_> {
    fn declared_bytes(&self) -> u64 {
        self.0.len() as u64
    }

    fn range(
        &self,
        offset: u64,
        bytes: usize,
    ) -> Result<&[u8], forever_db2::availability::Unavailable> {
        let start = usize::try_from(offset).map_err(|_| forever_db2::availability::Unavailable)?;
        let end = start
            .checked_add(bytes)
            .ok_or(forever_db2::availability::Unavailable)?;
        self.0
            .get(start..end)
            .ok_or(forever_db2::availability::Unavailable)
    }
}

pub(crate) fn authenticate_map(bytes: &[u8]) -> Result<AuthenticatedMap, Error> {
    let source = forever_db2::availability::read_map_70450(&SealedMapSource(bytes))
        .map_err(|_| Error::from("product_terrain_coverage_authentication"))?;
    require(source.sections().len() <= 16)
        .map_err(|_| Error::from("product_terrain_coverage_authentication"))?;
    let direct_rows = source.direct_records().len();
    let copy_records = source.copies().len();
    let mut sections = [None; 16];
    for (slot, section) in sections.iter_mut().zip(source.sections()) {
        *slot = Some(*section);
    }
    let materialized = source
        .materialize()
        .map_err(|_| Error::from("product_terrain_coverage_authentication"))?;
    let mut rows = Vec::new();
    rows.try_reserve_exact(materialized.rows().len())
        .map_err(|_| Error::from("product_allocation"))?;
    for row in materialized.rows() {
        let wdt_file_data_id = u32::try_from(row.fields.wdt_file_data_id)
            .ok()
            .filter(|id| *id != 0)
            .ok_or("product_terrain_coverage_authentication")?;
        rows.push((row.id, wdt_file_data_id));
    }
    Ok(AuthenticatedMap {
        declared_bytes: bytes.len() as u64,
        direct_rows,
        copy_records,
        materialized_rows: rows.len(),
        sections,
        rows,
    })
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
        if artifact.path == COVERAGE_PATH || artifact.path == COVERAGE_PATH_V24 {
            require(self.coverage.is_none())?;
            let coverage = Coverage::parse(bytes)?;
            if (artifact.path == COVERAGE_PATH_V24) != coverage.map_wdt_file_data_ids.is_some() {
                return Err("product_terrain_coverage_authentication".into());
            }
            self.coverage = Some(coverage);
        } else if artifact.path.ends_with(".tilelist") {
            let id = parse_list(&artifact.path)?;
            map_bits(&mut self.maps, id)?.tile_list(bytes)?;
        } else {
            let (id, x, y) = parse_tile(&artifact.path)?;
            forever_terrain::maps::verify_70450(bytes, 70450, forever_terrain::maps::MAX_MAP_BYTES)
                .map_err(|error| Error::from(error.code()))?;
            map_bits(&mut self.maps, id)?.tile(x, y)?;
        }
        Ok(())
    }
    pub fn finish(self, product: &Plan, context: Context) -> Result<(), Error> {
        self.finish_inner(product, context, None)
    }

    pub(crate) fn finish_authenticated(
        self,
        product: &Plan,
        context: Context,
        map: &AuthenticatedMap,
    ) -> Result<(), Error> {
        if !product.is_v24() {
            return Err("product_terrain_coverage_authentication".into());
        }
        self.finish_inner(product, context, Some(map))
    }

    fn finish_inner(
        self,
        product: &Plan,
        context: Context,
        authenticated_map: Option<&AuthenticatedMap>,
    ) -> Result<(), Error> {
        let coverage = self.coverage.ok_or("product_terrain_coverage")?;
        if product.is_v24() != coverage.map_wdt_file_data_ids.is_some() {
            return Err("product_terrain_coverage_authentication".into());
        }
        require(
            coverage.context == context
                && coverage.selection == product.terrain
                && coverage.pack == product.pack
                && self.maps.len() == coverage.maps.len(),
        )?;
        for outcome in &coverage.maps {
            let index = self
                .maps
                .binary_search_by_key(&outcome.map_id, |map| map.id)
                .map_err(|_| Error::from("product_terrain_coverage"))?;
            let actual = &self.maps[index];
            require(
                actual.list
                    && actual.maps == actual.listed
                    && actual.count() as usize == outcome.converted_tiles,
            )?;
        }
        if product.is_v24() {
            let map = authenticated_map.ok_or("product_terrain_coverage_authentication")?;
            authenticate_coverage_map(&coverage, map)?;
        } else if authenticated_map.is_some() {
            return Err("product_terrain_coverage_authentication".into());
        }
        Ok(())
    }
}

fn authenticate_coverage_map(coverage: &Coverage, map: &AuthenticatedMap) -> Result<(), Error> {
    let matches_facts = coverage.map.declared_bytes == map.declared_bytes
        && coverage.map.direct_rows == map.direct_rows
        && coverage.map.copy_records == map.copy_records
        && coverage.map.materialized_rows == map.materialized_rows
        && coverage.map.sections == map.sections;
    if !matches_facts {
        return Err("product_terrain_coverage_authentication".into());
    }
    let identities = coverage
        .map_wdt_file_data_ids
        .as_deref()
        .ok_or("product_terrain_coverage_authentication")?;
    match coverage.selection {
        Selection::All => {
            if coverage.maps.len() != map.rows.len() || identities.len() != map.rows.len() {
                return Err("product_terrain_coverage_authentication".into());
            }
            for (ordinal, ((outcome, identity), (map_id, wdt_file_data_id))) in coverage
                .maps
                .iter()
                .zip(identities)
                .zip(&map.rows)
                .enumerate()
            {
                if *identity != (*map_id, *wdt_file_data_id)
                    || *wdt_file_data_id == 0
                    || outcome.plan_index != ordinal
                    || outcome.materialized_ordinal != ordinal
                    || outcome.map_id != *map_id
                {
                    return Err("product_terrain_coverage_authentication".into());
                }
            }
        }
        Selection::Map(selected) => {
            let mut matches =
                map.rows
                    .iter()
                    .enumerate()
                    .filter(|(_, (map_id, wdt_file_data_id))| {
                        *map_id == selected && *wdt_file_data_id != 0
                    });
            let (ordinal, (map_id, wdt_file_data_id)) = matches
                .next()
                .ok_or("product_terrain_coverage_authentication")?;
            if matches.next().is_some()
                || coverage.maps.len() != 1
                || identities.len() != 1
                || identities[0] != (*map_id, *wdt_file_data_id)
                || coverage.maps[0].plan_index != 0
                || coverage.maps[0].materialized_ordinal != ordinal
                || coverage.maps[0].map_id != *map_id
            {
                return Err("product_terrain_coverage_authentication".into());
            }
        }
        _ => return Err("product_terrain_coverage_authentication".into()),
    }
    Ok(())
}

pub fn admit_metadata(product: &Plan, artifacts: &[Artifact]) -> Result<(), Error> {
    if !product.is_v20() {
        return Err("product_terrain_profile".into());
    }
    admit_metadata_with_coverage_path(product, artifacts, COVERAGE_PATH)
}

fn admit_metadata_with_coverage_path(
    product: &Plan,
    artifacts: &[Artifact],
    coverage_path: &str,
) -> Result<(), Error> {
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
    for artifact in artifacts {
        require(artifact.component == Component::Terrain && artifact.bytes > 0)?;
        total = total
            .checked_add(artifact.bytes)
            .ok_or("product_file_limit")?;
        require(total <= OUTPUT_BYTES)?;
        if artifact.path == coverage_path {
            require(!coverage && artifact.bytes <= COVERAGE_BYTES as u64)?;
            coverage = true;
        } else if artifact.path.ends_with(".tilelist") {
            require(artifact.bytes == tilelist::BYTES as u64 && lists.len() < MAX_MAPS)?;
            lists.push(parse_list(&artifact.path)?);
        } else {
            require(artifact.bytes <= forever_terrain::maps::MAX_MAP_BYTES as u64)?;
            let (id, _, _) = parse_tile(&artifact.path)?;
            if !tiles.contains(&id) {
                require(tiles.len() < MAX_MAPS)?;
                tiles.push(id);
            }
        }
    }
    lists.sort_unstable();
    require(
        coverage
            && lists.windows(2).all(|pair| pair[0] != pair[1])
            && tiles.iter().all(|id| lists.binary_search(id).is_ok()),
    )?;
    if let Selection::Map(id) = product.terrain {
        require(lists == [id])?;
    }
    Ok(())
}

pub fn admit_metadata_v24(product: &Plan, artifacts: &[Artifact]) -> Result<(), Error> {
    if !product.is_v24() {
        return Err("product_terrain_profile".into());
    }
    let map_path = format!("dbc/{}/Map.db2", product.locale.name());
    let mut map = false;
    let mut terrain = Vec::new();
    terrain
        .try_reserve_exact(artifacts.len())
        .map_err(|_| Error::from("product_allocation"))?;
    for artifact in artifacts {
        match artifact.component {
            Component::Db2 => {
                require(
                    !map && artifact.path == map_path
                        && artifact.bytes > 0
                        && artifact.bytes <= 16 * 1024 * 1024,
                )?;
                map = true;
            }
            Component::Terrain => terrain.push(artifact.clone()),
            _ => return Err("product_terrain_coverage".into()),
        }
    }
    require(map)?;
    admit_metadata_with_coverage_path(product, &terrain, COVERAGE_PATH_V24)
}

#[cfg(test)]
mod tests {
    use super::*;
    use forever_casc::{
        Locale, integrity, local_index, root,
        terrain::run::{MAP_ID, MATERIAL_ID, OBJECT_ID, TYPE_ID},
    };
    use std::{
        fs,
        path::PathBuf,
        sync::atomic::{AtomicU64, Ordering},
    };

    const MAP_COLUMNS: &[(u16, u16)] = &[
        (32, 1),
        (32, 1),
        (32, 1),
        (32, 1),
        (32, 1),
        (32, 1),
        (32, 2),
        (8, 1),
        (8, 1),
        (8, 1),
        (16, 1),
        (16, 1),
        (16, 1),
        (16, 1),
        (16, 1),
        (8, 1),
        (32, 1),
        (16, 1),
        (8, 1),
        (16, 1),
        (32, 1),
        (32, 1),
        (32, 1),
        (32, 1),
        (32, 1),
        (32, 3),
    ];
    const TYPE_COLUMNS: &[(u16, u16)] = &[
        (32, 1),
        (32, 6),
        (32, 1),
        (8, 1),
        (32, 1),
        (32, 1),
        (32, 1),
        (32, 1),
        (32, 1),
        (32, 1),
        (16, 1),
        (32, 1),
        (8, 1),
        (8, 1),
        (8, 1),
        (32, 1),
        (8, 6),
        (32, 3),
        (32, 38),
        (32, 4),
        (32, 4),
    ];
    static NEXT_LAB: AtomicU64 = AtomicU64::new(0);

    struct Lab {
        root: PathBuf,
        client: PathBuf,
        output: PathBuf,
    }
    impl Lab {
        fn new(with_wdt: bool) -> Self {
            let root_path = std::env::temp_dir().join(format!(
                "rce-v20-terrain-synthetic-{}-{}",
                std::process::id(),
                NEXT_LAB.fetch_add(1, Ordering::Relaxed)
            ));
            let client = root_path.join("client");
            let output = root_path.join("output");
            fs::create_dir_all(&client).unwrap();
            write_synthetic_client(&client, with_wdt);
            Self {
                root: root_path,
                client,
                output,
            }
        }
        fn new_v24(with_wdt: bool) -> Self {
            let root_path = std::env::temp_dir().join(format!(
                "rce-v24-terrain-synthetic-{}-{}",
                std::process::id(),
                NEXT_LAB.fetch_add(1, Ordering::Relaxed)
            ));
            let client = root_path.join("client");
            let output = root_path.join("output");
            fs::create_dir_all(&client).unwrap();
            write_synthetic_client_v24(&client, with_wdt);
            Self {
                root: root_path,
                client,
                output,
            }
        }
        fn run(&self) -> forever_casc::package::Summary {
            let product = Plan::parse(include_str!(
                "../../../../tests/contracts/fixtures/forever-product-v20/map2.tsv"
            ))
            .unwrap();
            crate::driver::extract(&product, &self.client, &self.output, [0x42; 32], false).unwrap()
        }
        fn run_v24(&self) -> forever_casc::package::Summary {
            let product = Plan::parse(include_str!(
                "../../../../tests/contracts/fixtures/forever-product-v24/map2.tsv"
            ))
            .unwrap();
            crate::driver::extract(&product, &self.client, &self.output, [0x42; 32], false).unwrap()
        }
    }
    impl Drop for Lab {
        fn drop(&mut self) {
            fs::remove_dir_all(&self.root).unwrap();
        }
    }

    #[derive(Clone)]
    struct Route {
        ckey: [u8; 16],
        size: u32,
        ekey: [u8; 16],
    }
    struct Record {
        ekey: [u8; 16],
        body: Vec<u8>,
    }

    fn put32(bytes: &mut [u8], at: usize, value: u32) {
        bytes[at..at + 4].copy_from_slice(&value.to_le_bytes());
    }
    fn width_offset(fields: &[(u16, u16)], index: usize) -> usize {
        fields[..index]
            .iter()
            .map(|(bits, count)| usize::from(bits * count / 8))
            .sum()
    }
    fn width_size(fields: &[(u16, u16)]) -> usize {
        width_offset(fields, fields.len())
    }
    fn wdc_table(
        layout: u32,
        hash: u32,
        fields: &[(u16, u16)],
        rows: &[(u32, Vec<u8>)],
        strings: &[u8],
    ) -> Vec<u8> {
        let data_start = 244 + fields.len() * 28;
        let mut bytes = vec![0; data_start];
        bytes[..4].copy_from_slice(b"WDC5");
        for (offset, value) in [
            (4, 5),
            (136, rows.len() as u32),
            (140, fields.len() as u32),
            (144, width_size(fields) as u32),
            (148, strings.len() as u32),
            (152, hash),
            (156, layout),
            (164, u32::MAX),
            (176, fields.len() as u32),
            (180, width_size(fields) as u32),
            (188, (fields.len() * 24) as u32),
            (200, 1),
            (212, data_start as u32),
            (216, rows.len() as u32),
            (220, strings.len() as u32),
            (228, (rows.len() * 4) as u32),
            (240, 0),
        ] {
            put32(&mut bytes, offset, value);
        }
        bytes[172..174].copy_from_slice(&4u16.to_le_bytes());
        bytes[174..176].copy_from_slice(&u16::MAX.to_le_bytes());
        for (index, &(bits, count)) in fields.iter().enumerate() {
            let primitive = 244 + index * 4;
            bytes[primitive..primitive + 2].copy_from_slice(&(32 - bits).to_le_bytes());
            bytes[primitive + 2..primitive + 4]
                .copy_from_slice(&(width_offset(fields, index) as u16).to_le_bytes());
            let column = 244 + fields.len() * 4 + index * 24;
            bytes[column..column + 2]
                .copy_from_slice(&((width_offset(fields, index) * 8) as u16).to_le_bytes());
            bytes[column + 2..column + 4].copy_from_slice(&(bits * count).to_le_bytes());
        }
        for (_, row) in rows {
            assert_eq!(row.len(), width_size(fields));
            bytes.extend_from_slice(row);
        }
        bytes.extend_from_slice(strings);
        for (id, _) in rows {
            bytes.extend_from_slice(&id.to_le_bytes());
        }
        bytes
    }
    fn map_db2() -> Vec<u8> {
        let fields = MAP_COLUMNS;
        let mut row = vec![0; width_size(fields)];
        let mut strings = Vec::new();
        for (field, text) in [(0, "Synthetic"), (1, "Name")] {
            let at = width_offset(fields, field);
            let relative = width_size(fields) + strings.len() - at;
            put32(&mut row, at, relative as u32);
            strings.extend_from_slice(text.as_bytes());
            strings.push(0);
        }
        put32(&mut row, width_offset(fields, 21), 100);
        for field in [13, 14] {
            let at = width_offset(fields, field);
            row[at..at + 2].copy_from_slice(&(-1i16).to_le_bytes());
        }
        let mut bytes = wdc_table(0xD43A_FAC3, 0xBD84_CD62, fields, &[(2, row)], &strings);
        bytes[172..174].copy_from_slice(&4u16.to_le_bytes());
        bytes
    }
    fn material_db2() -> Vec<u8> {
        wdc_table(
            0x98E5_D7AA,
            0x8C37_9329,
            &[(32, 1), (8, 1)],
            &[(7, vec![0; 5])],
            &[],
        )
    }
    fn object_db2() -> Vec<u8> {
        wdc_table(
            0xCB0D_39E8,
            0xFC2A_0DFF,
            &[(32, 1), (32, 1), (16, 1), (8, 1), (8, 1)],
            &[(1, vec![0; 12])],
            &[],
        )
    }
    fn type_db2() -> Vec<u8> {
        let mut row = vec![0; width_size(TYPE_COLUMNS)];
        row[width_offset(TYPE_COLUMNS, 14)] = 7;
        wdc_table(0xD1EC_EEC9, 0x6613_BED3, TYPE_COLUMNS, &[(22, row)], &[])
    }
    fn unknown_tact_map(mut bytes: Vec<u8>) -> Vec<u8> {
        let old_start = u32::from_le_bytes(bytes[212..216].try_into().unwrap()) as usize;
        let record_size = u32::from_le_bytes(bytes[144..148].try_into().unwrap()) as usize;
        for at in [old_start, old_start + 4] {
            let pointer = u32::from_le_bytes(bytes[at..at + 4].try_into().unwrap());
            put32(&mut bytes, at, pointer + record_size as u32);
        }
        let mut output = bytes[..244].to_vec();
        output.extend([0; 40]);
        output.extend(&bytes[244..old_start]);
        output.extend(0u32.to_le_bytes());
        output.extend(&bytes[old_start..]);
        put32(&mut output, 136, 2);
        put32(&mut output, 200, 2);
        put32(&mut output, 212, (old_start + 44) as u32);
        let data_at = output.len() as u32;
        output[244..252].copy_from_slice(&1u64.to_le_bytes());
        put32(&mut output, 252, data_at);
        put32(&mut output, 256, 1);
        put32(&mut output, 268, 4);
        put32(&mut output, 280, 0);
        output.extend(vec![0xA5; record_size]);
        output.extend(99u32.to_le_bytes());
        output
    }
    fn frame(tag: &[u8; 4], payload: &[u8]) -> Vec<u8> {
        let mut output = tag.iter().rev().copied().collect::<Vec<_>>();
        output.extend_from_slice(&(payload.len() as u32).to_le_bytes());
        output.extend_from_slice(payload);
        output
    }
    fn wdt() -> Vec<u8> {
        let mut output = frame(b"MVER", &18u32.to_le_bytes());
        let mut mphd = [0; 32];
        put32(&mut mphd, 0, 0x200);
        output.extend(frame(b"MPHD", &mphd));
        let mut main = vec![0; 64 * 64 * 8];
        let mut maid = vec![0; 64 * 64 * 32];
        let cell = 2 * 64 + 3;
        put32(&mut main, cell * 8, 1);
        put32(&mut main, cell * 8 + 4, 1234);
        put32(&mut maid, cell * 32, 201);
        output.extend(frame(b"MAIN", &main));
        output.extend(frame(b"MAID", &maid));
        output
    }
    fn adt() -> Vec<u8> {
        let mut output = frame(b"MVER", &18u32.to_le_bytes());
        let mut cell = vec![0; 128];
        put32(&mut cell, 52, 42);
        put32(&mut cell, 112, 1.0f32.to_bits());
        output.extend(frame(b"MCNK", &cell));
        output
    }

    fn partial_blte(decoded: &[u8], hole: std::ops::Range<usize>) -> ([u8; 16], Vec<u8>) {
        assert!(hole.start > 0 && hole.start < hole.end && hole.end < decoded.len());
        let mut blocks = Vec::<(Vec<u8>, usize)>::new();
        let mut first = vec![b'N'];
        first.extend_from_slice(&decoded[..hole.start]);
        blocks.push((first, hole.start));
        let mut encrypted = vec![b'E', 8];
        encrypted.extend([0x11; 8]);
        encrypted.push(4);
        encrypted.extend([0; 4]);
        encrypted.extend([b'S', 0x22]);
        blocks.push((encrypted, hole.end - hole.start));
        let mut last = vec![b'N'];
        last.extend_from_slice(&decoded[hole.end..]);
        blocks.push((last, decoded.len() - hole.end));
        let header_size = 12 + blocks.len() * 24;
        let mut header = b"BLTE".to_vec();
        header.extend_from_slice(&(header_size as u32).to_be_bytes());
        header.push(15);
        header.extend_from_slice(&(blocks.len() as u32).to_be_bytes()[1..]);
        for (block, decoded_size) in &blocks {
            header.extend_from_slice(&(block.len() as u32).to_be_bytes());
            header.extend_from_slice(&(*decoded_size as u32).to_be_bytes());
            header.extend_from_slice(&md5(&block[..]));
        }
        let ekey = md5(&header);
        for (block, _) in blocks {
            header.extend_from_slice(&block);
        }
        (ekey, header)
    }

    fn write_synthetic_client(client: &std::path::Path, with_wdt: bool) {
        write_synthetic_client_inner(client, with_wdt, true);
    }
    fn write_synthetic_client_v24(client: &std::path::Path, with_wdt: bool) {
        write_synthetic_client_inner(client, with_wdt, false);
    }
    fn write_synthetic_client_inner(client: &std::path::Path, with_wdt: bool, partial_map: bool) {
        let mut map = if partial_map {
            unknown_tact_map(map_db2())
        } else {
            map_db2()
        };
        put32(&mut map, 152, 0xBD84_CD62);
        let map_hole = if partial_map {
            let hole_start = u32::from_le_bytes(map[252..256].try_into().unwrap()) as usize;
            let hole_len = u32::from_le_bytes(map[144..148].try_into().unwrap()) as usize;
            Some(hole_start..hole_start + hole_len)
        } else {
            None
        };
        let mut inputs = vec![
            (MAP_ID, map, map_hole),
            (MATERIAL_ID, material_db2(), None),
            (OBJECT_ID, object_db2(), None),
            (TYPE_ID, type_db2(), None),
        ];
        if with_wdt {
            inputs.push((100, wdt(), None));
        }
        inputs.push((201, adt(), None));

        let mut source_routes = Vec::<Route>::new();
        let mut source_records = Vec::<Record>::new();
        let mut root_rows = Vec::<(u32, [u8; 16])>::new();
        for (ordinal, (id, data, hole)) in inputs.into_iter().enumerate() {
            let ckey = md5(&data);
            let (ekey, body) = match hole {
                Some(range) => partial_blte(&data, range),
                None => {
                    let mut body = b"BLTE\0\0\0\0N".to_vec();
                    body.extend_from_slice(&data);
                    ([0x60u8.wrapping_add(ordinal as u8); 16], body)
                }
            };
            source_routes.push(Route {
                ckey,
                size: data.len() as u32,
                ekey,
            });
            source_records.push(Record { ekey, body });
            root_rows.push((id, ckey));
        }

        let root_bytes = root_file(&root_rows);
        let root_ckey = md5(&root_bytes);
        let root_ekey = [0x33; 16];
        let mut routes = vec![Route {
            ckey: root_ckey,
            size: root_bytes.len() as u32,
            ekey: root_ekey,
        }];
        routes.extend(source_routes);
        let encoding = encoding_file(&routes);
        let encoding_ckey = md5(&encoding);
        let encoding_ekey = [0x44; 16];

        let mut archive = Vec::new();
        let mut index_rows = Vec::<[u8; 18]>::new();
        for record in source_records.into_iter().chain([
            Record {
                ekey: root_ekey,
                body: headerless_n(&root_bytes),
            },
            Record {
                ekey: encoding_ekey,
                body: headerless_n(&encoding),
            },
        ]) {
            let offset = archive.len() as u64;
            archive.extend([0xA5; 30]);
            archive.extend_from_slice(&record.body);
            index_rows.push(index_entry(
                record.ekey,
                offset,
                (archive.len() as u64 - offset) as u32,
            ));
        }

        let data_dir = client.join("Data/data");
        fs::create_dir_all(&data_dir).unwrap();
        fs::write(data_dir.join("data.000"), &archive).unwrap();
        for bucket in 0..16u8 {
            let rows = if bucket == 0 {
                index_rows.clone()
            } else {
                vec![index_entry([0xE0u8.wrapping_add(bucket); 16], 0, 39)]
            };
            fs::write(
                data_dir.join(format!("{bucket:02x}00000001.idx")),
                index_file(bucket, &rows),
            )
            .unwrap();
        }

        let config = format!(
            "root = {}\nencoding = {} {}\nencoding-size = {} {}\nbuild-uid = wow_classic_beta\nbuild-name = synthetic-70450\n",
            hex16(root_ckey),
            hex16(encoding_ckey),
            hex16(encoding_ekey),
            encoding.len(),
            encoding.len() + 39,
        )
        .into_bytes();
        let config_ckey = md5(&config);
        let config_hex = hex16(config_ckey);
        let config_path = client
            .join("Data/config")
            .join(&config_hex[..2])
            .join(&config_hex[2..4])
            .join(&config_hex);
        fs::create_dir_all(config_path.parent().unwrap()).unwrap();
        fs::write(config_path, config).unwrap();
        fs::write(
            client.join(".build.info"),
            format!(
                "Product!STRING:0|Active!DEC:1|Version!STRING:0|Build Key!HEX:16|CDN Key!HEX:16|Tags!STRING:0\nwow_classic_beta|1|1.60.1.70450|{}|{}|enUS\n",
                hex16(config_ckey),
                hex16([0x55; 16]),
            ),
        )
        .unwrap();
    }
    fn headerless_n(bytes: &[u8]) -> Vec<u8> {
        let mut body = b"BLTE\0\0\0\0N".to_vec();
        body.extend_from_slice(bytes);
        body
    }
    fn root_file(rows: &[(u32, [u8; 16])]) -> Vec<u8> {
        let mut bytes = b"TSFM".to_vec();
        for value in [20, 2, rows.len() as u32, 0] {
            bytes.extend_from_slice(&value.to_le_bytes());
        }
        for &(id, ckey) in rows {
            for value in [1u32, 2, 0x1000_0000, 0] {
                bytes.extend_from_slice(&value.to_le_bytes());
            }
            bytes.push(0);
            bytes.extend_from_slice(&id.to_le_bytes());
            bytes.extend_from_slice(&ckey);
        }
        bytes
    }
    fn encoding_file(routes: &[Route]) -> Vec<u8> {
        let mut page = Vec::new();
        for route in routes {
            page.extend_from_slice(&1u16.to_le_bytes());
            page.extend_from_slice(&route.size.to_be_bytes());
            page.extend_from_slice(&route.ckey);
            page.extend_from_slice(&route.ekey);
        }
        assert!(page.len() < 1024);
        page.resize(1024, 0);
        let mut bytes = b"EN\x01\x10\x10".to_vec();
        bytes.extend_from_slice(&1u16.to_be_bytes());
        bytes.extend_from_slice(&1u16.to_be_bytes());
        bytes.extend_from_slice(&1u32.to_be_bytes());
        bytes.extend_from_slice(&0u32.to_be_bytes());
        bytes.push(0);
        bytes.extend_from_slice(&0u32.to_be_bytes());
        bytes.extend_from_slice(&routes[0].ckey);
        bytes.extend_from_slice(&md5(&page));
        bytes.extend_from_slice(&page);
        bytes
    }
    fn index_entry(ekey: [u8; 16], offset: u64, size: u32) -> [u8; 18] {
        let mut row = [0; 18];
        row[..9].copy_from_slice(&ekey[..9]);
        row[9..14].copy_from_slice(&offset.to_be_bytes()[3..]);
        row[14..].copy_from_slice(&size.to_le_bytes());
        row
    }
    fn index_file(bucket: u8, rows: &[[u8; 18]]) -> Vec<u8> {
        let mut bytes = vec![0; 32];
        bytes[..4].copy_from_slice(&16u32.to_le_bytes());
        bytes[8..10].copy_from_slice(&7u16.to_le_bytes());
        bytes[10] = bucket;
        bytes[12..16].copy_from_slice(&[4, 5, 9, 30]);
        let header_checksum = lookup3(&bytes[8..24], 0, 0).0;
        bytes[4..8].copy_from_slice(&header_checksum.to_le_bytes());
        bytes.extend_from_slice(&(rows.len() as u32 * 18).to_le_bytes());
        let mut checksum = (0, 0);
        for row in rows {
            checksum = lookup3(row, checksum.0, checksum.1);
        }
        bytes.extend_from_slice(&checksum.0.to_le_bytes());
        for row in rows {
            bytes.extend_from_slice(row);
        }
        bytes
    }
    fn hex16(key: [u8; 16]) -> String {
        key.iter().map(|byte| format!("{byte:02x}")).collect()
    }

    // Test-only lookup3 checksum used to author contiguous local index fixtures.
    // Original lookup3.c by Bob Jenkins (May 2006) is public domain, with no warranty.
    fn lookup3(bytes: &[u8], primary: u32, secondary: u32) -> (u32, u32) {
        let initial = 0xDEAD_BEEF_u32
            .wrapping_add(bytes.len() as u32)
            .wrapping_add(primary);
        let [mut a, mut b, mut c] = [initial, initial, initial.wrapping_add(secondary)];
        let mut remaining = bytes;
        while remaining.len() > 12 {
            a = a.wrapping_add(u32::from_le_bytes(remaining[..4].try_into().unwrap()));
            b = b.wrapping_add(u32::from_le_bytes(remaining[4..8].try_into().unwrap()));
            c = c.wrapping_add(u32::from_le_bytes(remaining[8..12].try_into().unwrap()));
            a = a.wrapping_sub(c) ^ c.rotate_left(4);
            c = c.wrapping_add(b);
            b = b.wrapping_sub(a) ^ a.rotate_left(6);
            a = a.wrapping_add(c);
            c = c.wrapping_sub(b) ^ b.rotate_left(8);
            b = b.wrapping_add(a);
            a = a.wrapping_sub(c) ^ c.rotate_left(16);
            c = c.wrapping_add(b);
            b = b.wrapping_sub(a) ^ a.rotate_left(19);
            a = a.wrapping_add(c);
            c = c.wrapping_sub(b) ^ b.rotate_left(4);
            b = b.wrapping_add(a);
            remaining = &remaining[12..];
        }
        if remaining.is_empty() {
            return (c, b);
        }
        let mut tail = [0; 12];
        tail[..remaining.len()].copy_from_slice(remaining);
        a = a.wrapping_add(u32::from_le_bytes(tail[..4].try_into().unwrap()));
        b = b.wrapping_add(u32::from_le_bytes(tail[4..8].try_into().unwrap()));
        c = c.wrapping_add(u32::from_le_bytes(tail[8..12].try_into().unwrap()));
        c = (c ^ b).wrapping_sub(b.rotate_left(14));
        a = (a ^ c).wrapping_sub(c.rotate_left(11));
        b = (b ^ a).wrapping_sub(a.rotate_left(25));
        c = (c ^ b).wrapping_sub(b.rotate_left(16));
        a = (a ^ c).wrapping_sub(c.rotate_left(4));
        b = (b ^ a).wrapping_sub(a.rotate_left(14));
        c = (c ^ b).wrapping_sub(b.rotate_left(24));
        (c, b)
    }

    // Minimal test-only RFC 1321 MD5 for authored synthetic CASC CKeys and frame EKeys.
    fn md5(bytes: &[u8]) -> [u8; 16] {
        const K: [u32; 64] = [
            0xd76a_a478,
            0xe8c7_b756,
            0x2420_70db,
            0xc1bd_ceee,
            0xf57c_0faf,
            0x4787_c62a,
            0xa830_4613,
            0xfd46_9501,
            0x6980_98d8,
            0x8b44_f7af,
            0xffff_5bb1,
            0x895c_d7be,
            0x6b90_1122,
            0xfd98_7193,
            0xa679_438e,
            0x49b4_0821,
            0xf61e_2562,
            0xc040_b340,
            0x265e_5a51,
            0xe9b6_c7aa,
            0xd62f_105d,
            0x0244_1453,
            0xd8a1_e681,
            0xe7d3_fbc8,
            0x21e1_cde6,
            0xc337_07d6,
            0xf4d5_0d87,
            0x455a_14ed,
            0xa9e3_e905,
            0xfcef_a3f8,
            0x676f_02d9,
            0x8d2a_4c8a,
            0xfffa_3942,
            0x8771_f681,
            0x6d9d_6122,
            0xfde5_380c,
            0xa4be_ea44,
            0x4bde_cfa9,
            0xf6bb_4b60,
            0xbebf_bc70,
            0x289b_7ec6,
            0xeaa1_27fa,
            0xd4ef_3085,
            0x0488_1d05,
            0xd9d4_d039,
            0xe6db_99e5,
            0x1fa2_7cf8,
            0xc4ac_5665,
            0xf429_2244,
            0x432a_ff97,
            0xab94_23a7,
            0xfc93_a039,
            0x655b_59c3,
            0x8f0c_cc92,
            0xffef_f47d,
            0x8584_5dd1,
            0x6fa8_7e4f,
            0xfe2c_e6e0,
            0xa301_4314,
            0x4e08_11a1,
            0xf753_7e82,
            0xbd3a_f235,
            0x2ad7_d2bb,
            0xeb86_d391,
        ];
        const S: [u32; 64] = [
            7, 12, 17, 22, 7, 12, 17, 22, 7, 12, 17, 22, 7, 12, 17, 22, 5, 9, 14, 20, 5, 9, 14, 20,
            5, 9, 14, 20, 5, 9, 14, 20, 4, 11, 16, 23, 4, 11, 16, 23, 4, 11, 16, 23, 4, 11, 16, 23,
            6, 10, 15, 21, 6, 10, 15, 21, 6, 10, 15, 21, 6, 10, 15, 21,
        ];
        let bit_len = (bytes.len() as u64).wrapping_mul(8);
        let mut padded = bytes.to_vec();
        padded.push(0x80);
        while padded.len() % 64 != 56 {
            padded.push(0);
        }
        padded.extend_from_slice(&bit_len.to_le_bytes());
        let mut state = [0x6745_2301u32, 0xEFCD_AB89, 0x98BA_DCFE, 0x1032_5476];
        for block in padded.as_chunks::<64>().0 {
            let mut words = [0u32; 16];
            for (index, word) in words.iter_mut().enumerate() {
                *word = u32::from_le_bytes(block[index * 4..index * 4 + 4].try_into().unwrap());
            }
            let [mut a, mut b, mut c, mut d] = state;
            for i in 0..64 {
                let (f, g) = match i {
                    0..=15 => ((b & c) | (!b & d), i),
                    16..=31 => ((d & b) | (!d & c), (5 * i + 1) % 16),
                    32..=47 => (b ^ c ^ d, (3 * i + 5) % 16),
                    _ => (c ^ (b | !d), (7 * i) % 16),
                };
                let next = a.wrapping_add(f).wrapping_add(K[i]).wrapping_add(words[g]);
                let old_d = d;
                d = c;
                c = b;
                b = b.wrapping_add(next.rotate_left(S[i]));
                a = old_d;
            }
            state[0] = state[0].wrapping_add(a);
            state[1] = state[1].wrapping_add(b);
            state[2] = state[2].wrapping_add(c);
            state[3] = state[3].wrapping_add(d);
        }
        let mut digest = [0; 16];
        for (index, word) in state.into_iter().enumerate() {
            digest[index * 4..index * 4 + 4].copy_from_slice(&word.to_le_bytes());
        }
        digest
    }

    #[test]
    fn synthetic_digest_helpers_match_authored_vectors() {
        assert_eq!(
            md5(b""),
            [
                0xd4, 0x1d, 0x8c, 0xd9, 0x8f, 0x00, 0xb2, 0x04, 0xe9, 0x80, 0x09, 0x98, 0xec, 0xf8,
                0x42, 0x7e,
            ]
        );
        assert_eq!(
            lookup3(b"Four score and seven years ago", 0, 0),
            (0x17770551, 0xce7226e6)
        );
    }

    fn partial_coverage() -> Coverage {
        let product = Plan::parse(include_str!(
            "../../../../tests/contracts/fixtures/forever-product-v20/map2.tsv"
        ))
        .unwrap();
        let mut sections = [None; 16];
        sections[0] = Some(SectionCoverage {
            ordinal: 0,
            declared_records: 1,
            declared_copies: 0,
            state: SectionState::Admitted,
        });
        sections[1] = Some(SectionCoverage {
            ordinal: 1,
            declared_records: 1,
            declared_copies: 0,
            state: SectionState::UnknownTact,
        });
        Coverage {
            context: Context {
                build: 70450,
                locale: product.locale,
                source_sha256: [1; 32],
                plan_sha256: integrity::digest(product.render().as_bytes()),
                producer_sha256: [2; 32],
                components: Component::Terrain.bit(),
            },
            selection: Selection::Map(2),
            pack: true,
            map: available::MapFacts {
                declared_bytes: 1000,
                available_bytes: 800,
                full_ckey_verified: false,
                encoded_key_authenticated: true,
                work: Default::default(),
                direct_rows: 1,
                copy_records: 0,
                materialized_rows: 1,
                sections,
            },
            maps: vec![available::CompatibleMapOutcome {
                map_id: 2,
                plan_index: 0,
                materialized_ordinal: 0,
                wdt: available::WdtDisposition::Loaded,
                known_present_tiles: Some(0),
                converted_tiles: 0,
                missing_root_adts: 0,
            }],
            map_wdt_file_data_ids: None,
        }
    }

    #[test]
    fn coverage_invariant_accepts_partial_auth_and_rejects_relaxed_claims() {
        let partial = partial_coverage();
        assert!(!partial.map.full_ckey_verified);
        assert!(partial.map.encoded_key_authenticated);
        assert!(partial.map.available_bytes < partial.map.declared_bytes);
        assert!(partial.render().is_ok());

        let mut false_full = partial.clone();
        false_full.map.full_ckey_verified = true;
        assert!(false_full.render().is_err());
        let mut unauthenticated_partial = partial;
        unauthenticated_partial.map.encoded_key_authenticated = false;
        assert!(unauthenticated_partial.render().is_err());
    }

    #[test]
    fn v20_coverage_stays_mask_eight_and_cannot_claim_v24_authentication() {
        let v20 = Plan::parse(include_str!(
            "../../../../tests/contracts/fixtures/forever-product-v20/map2.tsv"
        ))
        .unwrap();
        let mut mask_nine = partial_coverage();
        mask_nine.context.components = Component::Db2.bit() | Component::Terrain.bit();
        assert_eq!(
            mask_nine.render().unwrap_err().code,
            "product_terrain_coverage"
        );

        let mut v24_coverage = partial_coverage();
        v24_coverage.context.components = Component::Db2.bit() | Component::Terrain.bit();
        v24_coverage.map_wdt_file_data_ids = Some(vec![(2, 100)]);
        let v24_bytes = v24_coverage.render().unwrap();
        assert!(v24_bytes.starts_with("terrain-coverage\t3\n"));
        let mut facts = VerifyFacts::new();
        facts.coverage = Some(Coverage::parse(v24_bytes.as_bytes()).unwrap());
        assert_eq!(
            facts.finish(&v20, v24_coverage.context).unwrap_err().code,
            "product_terrain_coverage_authentication"
        );

        assert!(
            Plan::parse(include_str!(
                "../../../../tests/contracts/fixtures/forever-product-v24/map2.tsv"
            ))
            .unwrap()
            .validate()
            .is_ok()
        );
        assert!(
            Plan {
                components: Component::Db2.bit() | Component::Terrain.bit(),
                ..v20
            }
            .validate()
            .is_err()
        );
    }

    #[test]
    fn v20_production_dispatch_accepts_partial_map_and_records_authentication_facts() {
        let lab = Lab::new(true);
        let summary = lab.run();
        assert_eq!(summary.context.build, 70450);
        assert_eq!(summary.context.components, Component::Terrain.bit());
        let bytes = fs::read(lab.output.join(COVERAGE_PATH)).unwrap();
        let coverage = Coverage::parse(&bytes).unwrap();
        assert!(!coverage.map.full_ckey_verified);
        assert!(coverage.map.encoded_key_authenticated);
        assert!(coverage.map.available_bytes < coverage.map.declared_bytes);
        assert_eq!(
            coverage.map.sections[1].unwrap().state,
            SectionState::UnknownTact
        );
        assert_eq!(coverage.maps.len(), 1);
        assert_eq!(coverage.maps[0].wdt, available::WdtDisposition::Loaded);
        crate::verify::verify(
            &Plan::parse(include_str!(
                "../../../../tests/contracts/fixtures/forever-product-v20/map2.tsv"
            ))
            .unwrap(),
            &lab.output,
            None,
        )
        .unwrap();
        crate::verify::verify_links(
            &Plan::parse(include_str!(
                "../../../../tests/contracts/fixtures/forever-product-v20/map2.tsv"
            ))
            .unwrap(),
            &lab.output,
            None,
        )
        .unwrap();
    }

    #[test]
    fn v24_seals_map_db2_with_v20_terrain_and_independently_verifies_it() {
        let lab = Lab::new_v24(true);
        let summary = lab.run_v24();
        assert_eq!(summary.context.build, 70450);
        assert_eq!(
            summary.context.components,
            Component::Db2.bit() | Component::Terrain.bit()
        );
        assert!(lab.output.join("dbc/enUS/Map.db2").is_file());
        let coverage =
            Coverage::parse(&fs::read(lab.output.join(COVERAGE_PATH_V24)).unwrap()).unwrap();
        assert_eq!(coverage.context.components, 9);
        assert_eq!(coverage.maps.len(), 1);
        let product = Plan::parse(include_str!(
            "../../../../tests/contracts/fixtures/forever-product-v24/map2.tsv"
        ))
        .unwrap();
        crate::verify::verify(&product, &lab.output, None).unwrap();
        crate::verify::verify_links(&product, &lab.output, None).unwrap();
    }

    #[test]
    fn v24_coverage_authentication_binds_sealed_map_identity_and_materialized_count() {
        let bytes = map_db2();
        let map = authenticate_map(&bytes).unwrap();
        assert_eq!(map.rows, [(2, 100)]);
        assert_eq!(map.materialized_rows, 1);

        let product = Plan::parse(include_str!(
            "../../../../tests/contracts/fixtures/forever-product-v24/map2.tsv"
        ))
        .unwrap();
        let coverage = Coverage {
            context: Context {
                build: 70450,
                locale: product.locale,
                source_sha256: [1; 32],
                plan_sha256: integrity::digest(product.render().as_bytes()),
                producer_sha256: [2; 32],
                components: 9,
            },
            selection: Selection::Map(2),
            pack: true,
            map: available::MapFacts {
                declared_bytes: map.declared_bytes,
                available_bytes: map.declared_bytes,
                full_ckey_verified: true,
                encoded_key_authenticated: false,
                work: Default::default(),
                direct_rows: map.direct_rows,
                copy_records: map.copy_records,
                materialized_rows: map.materialized_rows,
                sections: map.sections,
            },
            maps: vec![available::CompatibleMapOutcome {
                map_id: 2,
                plan_index: 0,
                materialized_ordinal: 0,
                wdt: available::WdtDisposition::Loaded,
                known_present_tiles: Some(0),
                converted_tiles: 0,
                missing_root_adts: 0,
            }],
            map_wdt_file_data_ids: Some(vec![(2, 100)]),
        };
        let rendered = coverage.render();
        assert!(rendered.is_ok(), "{}", rendered.unwrap_err().code);
        assert!(authenticate_coverage_map(&coverage, &map).is_ok());

        let mut wrong_identity = coverage.clone();
        wrong_identity.maps[0].map_id = 3;
        assert_eq!(
            authenticate_coverage_map(&wrong_identity, &map)
                .unwrap_err()
                .code,
            "product_terrain_coverage_authentication"
        );
        let mut wrong_count = coverage;
        wrong_count.map.materialized_rows += 1;
        assert_eq!(
            authenticate_coverage_map(&wrong_count, &map)
                .unwrap_err()
                .code,
            "product_terrain_coverage_authentication"
        );
    }

    #[test]
    fn v20_production_dispatch_is_compatible_not_strict_for_missing_wdt() {
        let lab = Lab::new(false);
        lab.run();
        let plan = Plan::parse(include_str!(
            "../../../../tests/contracts/fixtures/forever-product-v20/map2.tsv"
        ))
        .unwrap();
        let coverage = Coverage::parse(&fs::read(lab.output.join(COVERAGE_PATH)).unwrap()).unwrap();
        assert_eq!(coverage.maps[0].wdt, available::WdtDisposition::Unavailable);
        assert_eq!(coverage.maps[0].known_present_tiles, None);
        assert_eq!(coverage.maps[0].converted_tiles, 0);
        crate::verify::verify(&plan, &lab.output, None).unwrap();
        crate::verify::verify_links(&plan, &lab.output, None).unwrap();

        let mut store = open_store(&lab.client);
        let mut sink = EmptySink;
        let strict = available::run_full_maps(
            &mut store,
            plan::Selection::MapIds(&[2]),
            plan.pack,
            available::FullMapLimits::default(),
            &mut sink,
        );
        assert!(strict.is_err());
    }

    struct EmptySink;
    impl available::Sink for EmptySink {
        fn tile(
            &mut self,
            _: forever_terrain::conversion::Context,
            _: run::SourceFacts,
            _: &forever_terrain::conversion::ConvertedTile,
        ) -> Result<(), run::SinkError> {
            Ok(())
        }
    }

    fn open_store(client: &std::path::Path) -> LocalStore {
        LocalStore::open_db2_70450(
            client,
            forever_casc::local_store::OpenOptions {
                locale: Locale::EnUs,
                index_layouts: [local_index::Layout::Contiguous; 16],
                root_layout: root::Layout::Header50893V2,
                limits: Default::default(),
            },
        )
        .unwrap()
    }
}
