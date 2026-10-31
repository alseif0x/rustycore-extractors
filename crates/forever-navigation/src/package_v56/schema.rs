// SPDX-License-Identifier: GPL-3.0-or-later
// Added 2026-10-31: share the producer's canonical v56 coverage codec.

use super::Error;
use crate::{inventory::fs::run::AvailableLimits, tilebuilder::Coordinate};
use forever_casc::{
    integrity,
    package::{Component, Context, Summary},
};
use std::fmt::Write as _;

pub const COVERAGE_PATH: &str = "mmaps/available-navigation-v56.tsv";
pub const COVERAGE_MAX: usize = 128 * 1024 * 1024;
pub const MAPS_MAX: usize = 4096;
pub const TILES_MAX: usize = 4096 * 4096;
const OUTPUT_BYTES_MAX: u64 = 1024 * 1024 * 1024 * 1024;

pub use crate::package_v23::TileOutput;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MapFact {
    pub id: u32,
    pub root: u32,
    pub state: &'static str,
    pub tiles: Vec<Coordinate>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Output {
    pub map: u32,
    pub global: Option<TileOutput>,
    pub tiles: Vec<TileOutput>,
    pub partial_triangulations: u64,
    pub detail_center_fallbacks: u64,
    pub unset_height_samples: u64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Coverage56 {
    pub context: Context,
    pub source: Summary,
    pub maps: Vec<MapFact>,
    pub outputs: Vec<Output>,
    pub builder: u64,
    pub save: u64,
}

fn identity(source: &Summary) -> [u8; 32] {
    integrity::digest(
        format!("rustycore-navigation-v56-sealed-v51\n{}\n", source.json()).as_bytes(),
    )
}

impl Coverage56 {
    pub fn render(&self) -> Result<Vec<u8>, Error> {
        self.validate()?;
        let mut text = format!(
            "navigation-coverage\t56\ncontext\t{}\nsource-context\t{}\nsource\t{}\t{}\t{}\n",
            self.context.json(),
            self.source.context.json(),
            self.source.files,
            self.source.bytes,
            integrity::hex(&self.source.inventory_sha256)
        );
        for (map, output) in self.maps.iter().zip(&self.outputs) {
            writeln!(
                text,
                "map\t{}\t{}\t{}\t{}",
                map.id,
                map.root,
                map.state,
                map.tiles.len()
            )
            .expect("String formatting");
            if let Some(global) = &output.global {
                writeln!(
                    text,
                    "global\t{}\t{}",
                    global.bytes,
                    integrity::hex(&global.sha256)
                )
                .expect("String formatting");
            }
            for tile in &output.tiles {
                writeln!(
                    text,
                    "tile\t{}\t{}\t{}\t{}\t{}",
                    tile.tile.row,
                    tile.tile.column,
                    tile.written,
                    tile.bytes,
                    integrity::hex(&tile.sha256)
                )
                .expect("String formatting");
            }
            writeln!(text, "partials\t{}", output.partial_triangulations)
                .expect("String formatting");
            writeln!(text, "center-fallbacks\t{}", output.detail_center_fallbacks)
                .expect("String formatting");
            writeln!(text, "unset-heights\t{}", output.unset_height_samples)
                .expect("String formatting");
        }
        writeln!(text, "work\t{}\t{}", self.builder, self.save).expect("String formatting");
        if text.len() > COVERAGE_MAX {
            return Err("product_navigation_coverage_limit".into());
        }
        Ok(text.into_bytes())
    }

    pub fn validate(&self) -> Result<(), Error> {
        self.context.validate()?;
        self.source.context.validate()?;
        let tiles: usize = self.maps.iter().map(|map| map.tiles.len()).sum();
        let output_bytes = self
            .outputs
            .iter()
            .flat_map(|output| output.global.iter().chain(&output.tiles))
            .try_fold(0_u64, |total, tile| total.checked_add(tile.bytes))
            .filter(|total| *total <= OUTPUT_BYTES_MAX)
            .ok_or("product_navigation_coverage_limit")?;
        let estimated = self
            .maps
            .len()
            .checked_mul(256)
            .and_then(|size| {
                tiles
                    .checked_mul(128)
                    .and_then(|count| size.checked_add(count))
            })
            .and_then(|size| size.checked_add(4096))
            .filter(|size| *size <= COVERAGE_MAX)
            .ok_or("product_navigation_coverage_limit")?;
        let _ = (output_bytes, estimated);
        let limits = AvailableLimits::all_available_v56(self.maps.len(), tiles)
            .map_err(|error| Error::from(error.code()))?;
        if self.context.build != 70450
            || self.context.components != Component::Navigation.bit()
            || self.source.context.build != 70450
            || self.source.context.components != 25
            || self.source.context.locale != self.context.locale
            || self.context.source_sha256 != identity(&self.source)
            || self.source.files == 0
            || self.source.files > 262_144
            || self.maps.len() != self.outputs.len()
            || self.maps.len() > MAPS_MAX
            || tiles > TILES_MAX
            || self.builder > limits.builder_work
            || self.save > limits.save_work
            || self.maps.windows(2).any(|pair| pair[0].id >= pair[1].id)
        {
            return Err("product_navigation_coverage".into());
        }
        for (map, output) in self.maps.iter().zip(&self.outputs) {
            if map.id != output.map
                || map.id > 9999
                || map.root > 9999
                || map.tiles.len() > 4096
                || map.tiles.len() != output.tiles.len()
                || map
                    .tiles
                    .windows(2)
                    .any(|pair| (pair[0].row, pair[0].column) >= (pair[1].row, pair[1].column))
                || ((map.state == "planned") != (!map.tiles.is_empty() && output.global.is_some()))
                || (map.state != "planned"
                    && (!map.tiles.is_empty()
                        || output.global.is_some()
                        || output.partial_triangulations != 0
                        || output.detail_center_fallbacks != 0
                        || output.unset_height_samples != 0))
            {
                return Err("product_navigation_coverage".into());
            }
            if let Some(global) = output.global
                && (global.bytes == 0 || global.sha256 == [0; 32])
            {
                return Err("product_navigation_coverage".into());
            }
            for (coordinate, tile) in map.tiles.iter().zip(&output.tiles) {
                if coordinate != &tile.tile
                    || coordinate.row >= 64
                    || coordinate.column >= 64
                    || (tile.written && (tile.bytes == 0 || tile.sha256 == [0; 32]))
                    || (!tile.written && (tile.bytes != 0 || tile.sha256 != [0; 32]))
                {
                    return Err("product_navigation_coverage".into());
                }
            }
        }
        Ok(())
    }

    pub fn parse(bytes: &[u8]) -> Result<Self, Error> {
        if bytes.len() > COVERAGE_MAX || !bytes.is_ascii() {
            return Err("product_navigation_coverage_limit".into());
        }
        let text =
            std::str::from_utf8(bytes).map_err(|_| Error::from("product_navigation_coverage"))?;
        let mut lines = text.lines().peekable();
        if lines.next() != Some("navigation-coverage\t56") {
            return Err("product_navigation_coverage".into());
        }
        let context = Context::parse(next(&mut lines, "context", 2)?[1])?;
        let source_context = Context::parse(next(&mut lines, "source-context", 2)?[1])?;
        let fields = next(&mut lines, "source", 4)?;
        let source = Summary {
            context: source_context,
            files: num(fields[1])?,
            bytes: num(fields[2])?,
            inventory_sha256: sha(fields[3])?,
        };
        let mut maps = Vec::new();
        let mut outputs = Vec::new();
        let mut total = 0_usize;
        while lines.peek().is_some_and(|line| line.starts_with("map\t")) {
            if maps.len() >= MAPS_MAX {
                return Err("product_navigation_coverage_limit".into());
            }
            let fields = next(&mut lines, "map", 5)?;
            let id = num(fields[1])?;
            let root = num(fields[2])?;
            let state = match fields[3] {
                "planned" => "planned",
                "empty" => "empty",
                "root-absent" => "root-absent",
                _ => return Err("product_navigation_coverage".into()),
            };
            let count: usize = num(fields[4])?;
            total = total
                .checked_add(count)
                .filter(|count| *count <= TILES_MAX)
                .ok_or("product_navigation_coverage_limit")?;
            if count > 4096 {
                return Err("product_navigation_coverage_limit".into());
            }
            let global = if state == "planned" {
                let fields = next(&mut lines, "global", 3)?;
                Some(TileOutput {
                    tile: Coordinate { row: 0, column: 0 },
                    written: true,
                    bytes: num(fields[1])?,
                    sha256: sha(fields[2])?,
                })
            } else {
                None
            };
            let mut tiles = Vec::new();
            for _ in 0..count {
                let fields = next(&mut lines, "tile", 6)?;
                tiles.push(TileOutput {
                    tile: Coordinate {
                        row: num(fields[1])?,
                        column: num(fields[2])?,
                    },
                    written: match fields[3] {
                        "true" => true,
                        "false" => false,
                        _ => return Err("product_navigation_coverage".into()),
                    },
                    bytes: num(fields[4])?,
                    sha256: sha(fields[5])?,
                });
            }
            let partial_triangulations = num(next(&mut lines, "partials", 2)?[1])?;
            let detail_center_fallbacks = num(next(&mut lines, "center-fallbacks", 2)?[1])?;
            let unset_height_samples = num(next(&mut lines, "unset-heights", 2)?[1])?;
            maps.push(MapFact {
                id,
                root,
                state,
                tiles: tiles.iter().map(|tile| tile.tile).collect(),
            });
            outputs.push(Output {
                map: id,
                global,
                tiles,
                partial_triangulations,
                detail_center_fallbacks,
                unset_height_samples,
            });
        }
        let fields = next(&mut lines, "work", 3)?;
        let result = Self {
            context,
            source,
            maps,
            outputs,
            builder: num(fields[1])?,
            save: num(fields[2])?,
        };
        if lines.next().is_some() || result.render()? != bytes {
            return Err("product_navigation_coverage".into());
        }
        Ok(result)
    }
}

fn next<'a>(
    lines: &mut impl Iterator<Item = &'a str>,
    key: &str,
    count: usize,
) -> Result<Vec<&'a str>, Error> {
    let fields: Vec<_> = lines
        .next()
        .ok_or("product_navigation_coverage")?
        .split('\t')
        .collect();
    if fields.len() != count || fields[0] != key {
        return Err("product_navigation_coverage".into());
    }
    Ok(fields)
}

fn num<T: std::str::FromStr>(text: &str) -> Result<T, Error> {
    text.parse()
        .map_err(|_| Error::from("product_navigation_coverage"))
}

pub fn sha(text: &str) -> Result<[u8; 32], Error> {
    if text.len() != 64 || !text.is_ascii() {
        return Err("product_navigation_coverage".into());
    }
    let mut output = [0; 32];
    for (index, byte) in output.iter_mut().enumerate() {
        *byte = u8::from_str_radix(&text[index * 2..index * 2 + 2], 16)
            .map_err(|_| Error::from("product_navigation_coverage"))?;
    }
    Ok(output)
}
