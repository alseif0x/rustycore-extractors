// SPDX-License-Identifier: GPL-3.0-or-later
// Modified 2026-10-20: move selected-navigation package schema into its owner crate.
//! Canonical v23 identity and coverage encoding. These public structs carry
//! producer metadata; only `render`/`parse` validate the profile and canonical
//! representation. Their fields do not authenticate an origin or certify
//! source completeness.

use super::Error;
use crate::tilebuilder::Coordinate;
use forever_casc::{
    integrity,
    package::{Component, Context},
};
use forever_db2::catalog;
use std::collections::BTreeMap;

pub const COVERAGE_PATH: &str = "mmaps/selected-navigation-coverage-v1.tsv";
pub const COVERAGE_LIMIT: usize = 1024 * 1024;
pub const MAP_LIMIT: u64 = 16 * 1024 * 1024;
pub const MAP_RANGES_MAX: usize = 65_536;
pub const MAP_ROWS_MAX: usize = 100_000;
const OUTPUT_PROFILE: &str = "linux70450-selected-navigation-v1";
const IDENTITY_DOMAIN: &str = "rustycore-v23-selected-navigation-identity-v2";

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PackageIdentity {
    pub context: Context,
    pub plan_sha256: [u8; 32],
    pub files: u64,
    pub bytes: u64,
    pub inventory_sha256: [u8; 32],
    pub coverage_sha256: [u8; 32],
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MapSnapshot {
    pub file_data_id: u32,
    pub declared_bytes: u64,
    pub available_bytes: u64,
    pub full_ckey_verified: bool,
    pub encoded_key_authenticated: bool,
    pub range_count: usize,
    pub section_count: usize,
    pub declared_direct_rows: usize,
    pub declared_copy_pairs: usize,
    pub direct_rows: usize,
    pub copy_pairs: usize,
    /// Terrain projection after its WDT-zero filter and copy replay.
    pub terrain_materialized_rows: usize,
    /// VMap projection after copy replay; WDT-zero rows remain available.
    pub vmap_materialized_rows: usize,
    pub decoded_sha256: [u8; 32],
    pub snapshot_sha256: [u8; 32],
}

/// Checks the terrain-reader projection count, which applies its WDT-zero filter.
pub fn terrain_materialized_rows_match(snapshot: &MapSnapshot, rows: usize) -> bool {
    snapshot.terrain_materialized_rows == rows
}

/// Checks the VMap-reader projection count, which retains WDT-zero rows.
pub fn vmap_materialized_rows_match(snapshot: &MapSnapshot, rows: usize) -> bool {
    snapshot.vmap_materialized_rows == rows
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LiquidSnapshot {
    pub file_data_id: u32,
    pub bytes: u64,
    pub direct_rows: usize,
    pub copy_pairs: usize,
    pub sha256: [u8; 32],
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ClosureMap {
    pub id: u32,
    pub root: u32,
    pub parent: Option<u32>,
    pub cosmetic_parent: Option<u32>,
    pub wdt_file_data_id: u32,
    pub output: bool,
    pub is_parent: bool,
    pub tiles: usize,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TerrainMap {
    pub id: u32,
    pub loaded: bool,
    pub known_present: usize,
    pub converted: usize,
    pub missing_root_adts: usize,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SourceIdentity {
    pub selected_map: u32,
    pub build: u32,
    pub locale: String,
    pub plan_sha256: [u8; 32],
    pub producer_sha256: [u8; 32],
    pub current_source_sha256: [u8; 32],
    pub map: MapSnapshot,
    pub closure: Vec<ClosureMap>,
    pub liquid: LiquidSnapshot,
    pub terrain: PackageIdentity,
    pub terrain_maps: Vec<TerrainMap>,
    pub vmap: PackageIdentity,
}

impl SourceIdentity {
    pub fn render(&self) -> Result<String, Error> {
        if self.locale != "enUS" && self.locale != "esES" {
            return Err("product_navigation_coverage".into());
        }
        let mut out = String::new();
        out.try_reserve(128 * 1024)
            .map_err(|_| Error::from("product_allocation"))?;
        use std::fmt::Write as _;
        writeln!(out, "identity\t2\nprofile\t{IDENTITY_DOMAIN}").expect("String formatting");
        writeln!(out, "selected-map\t{}", self.selected_map).expect("String formatting");
        writeln!(out, "build\t{}\nlocale\t{}", self.build, self.locale).expect("String formatting");
        writeln!(out, "plan-sha256\t{}", integrity::hex(&self.plan_sha256))
            .expect("String formatting");
        writeln!(
            out,
            "producer-sha256\t{}",
            integrity::hex(&self.producer_sha256)
        )
        .expect("String formatting");
        writeln!(
            out,
            "current-source-sha256\t{}",
            integrity::hex(&self.current_source_sha256)
        )
        .expect("String formatting");
        writeln!(
            out,
            "map-snapshot\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}",
            self.map.file_data_id,
            self.map.declared_bytes,
            self.map.available_bytes,
            self.map.full_ckey_verified,
            self.map.encoded_key_authenticated,
            self.map.range_count,
            self.map.section_count,
            self.map.declared_direct_rows,
            self.map.declared_copy_pairs,
            self.map.direct_rows,
            self.map.copy_pairs,
            self.map.terrain_materialized_rows,
            self.map.vmap_materialized_rows,
            integrity::hex(&self.map.decoded_sha256),
            integrity::hex(&self.map.snapshot_sha256),
        )
        .expect("String formatting");
        for map in &self.closure {
            writeln!(
                out,
                "closure\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}",
                map.id,
                map.root,
                map.parent.map_or_else(|| "none".into(), |v| v.to_string()),
                map.cosmetic_parent
                    .map_or_else(|| "none".into(), |v| v.to_string()),
                map.wdt_file_data_id,
                map.output,
                map.is_parent,
                map.tiles,
            )
            .expect("String formatting");
        }
        writeln!(
            out,
            "liquid-type\t{}\t{}\t{}\t{}\t{}",
            self.liquid.file_data_id,
            self.liquid.bytes,
            self.liquid.direct_rows,
            self.liquid.copy_pairs,
            integrity::hex(&self.liquid.sha256),
        )
        .expect("String formatting");
        write_package_identity(&mut out, "terrain", &self.terrain);
        for map in &self.terrain_maps {
            writeln!(
                out,
                "terrain-map\t{}\t{}\t{}\t{}\t{}",
                map.id, map.loaded, map.known_present, map.converted, map.missing_root_adts
            )
            .expect("String formatting");
        }
        write_package_identity(&mut out, "vmap", &self.vmap);
        if out.len() > COVERAGE_LIMIT / 2 {
            return Err("product_navigation_coverage_limit".into());
        }
        Ok(out)
    }
}

fn write_package_identity(out: &mut String, name: &str, value: &PackageIdentity) {
    use std::fmt::Write as _;
    writeln!(out, "{name}-context\t{}", value.context.json()).expect("String formatting");
    writeln!(
        out,
        "{name}-plan-sha256\t{}",
        integrity::hex(&value.plan_sha256)
    )
    .expect("String formatting");
    writeln!(
        out,
        "{name}-package\t{}\t{}\t{}\t{}",
        value.files,
        value.bytes,
        integrity::hex(&value.inventory_sha256),
        integrity::hex(&value.coverage_sha256),
    )
    .expect("String formatting");
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TileOutput {
    pub tile: Coordinate,
    pub written: bool,
    pub bytes: u64,
    pub sha256: [u8; 32],
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Coverage {
    pub context: Context,
    pub identity: SourceIdentity,
    pub global_bytes: u64,
    pub global_sha256: [u8; 32],
    pub tiles: Vec<TileOutput>,
    pub maps: usize,
    pub written_tiles: usize,
    pub empty_tiles: usize,
    pub output_bytes: u64,
    pub builder_work: u64,
    pub save_work: u64,
    pub metadata_calls: u64,
}

impl Coverage {
    pub fn render(&self) -> Result<Vec<u8>, Error> {
        self.validate()?;
        let identity = self.identity.render()?;
        let identity_sha256 = integrity::digest(identity.as_bytes());
        if identity_sha256 != self.context.source_sha256 {
            return Err("product_navigation_identity".into());
        }
        let mut out = String::new();
        out.try_reserve_exact(identity.len() * 2 + 4096)
            .map_err(|_| Error::from("product_allocation"))?;
        use std::fmt::Write as _;
        writeln!(
            out,
            "navigation-coverage\t1\nprofile\t{OUTPUT_PROFILE}\ncontext\t{}\nidentity-sha256\t{}\nidentity-hex\t{}",
            self.context.json(),
            integrity::hex(&identity_sha256),
            hex_bytes(identity.as_bytes()),
        )
        .expect("String formatting");
        writeln!(
            out,
            "global\t{}\t{}\t{}",
            self.identity.selected_map,
            self.global_bytes,
            integrity::hex(&self.global_sha256),
        )
        .expect("String formatting");
        for tile in &self.tiles {
            writeln!(
                out,
                "tile\t{}\t{}\t{}\t{}\t{}",
                tile.tile.row,
                tile.tile.column,
                if tile.written { "written" } else { "empty" },
                tile.bytes,
                integrity::hex(&tile.sha256),
            )
            .expect("String formatting");
        }
        writeln!(
            out,
            "finish\t{}\t{}\t{}\t{}\t{}\t{}\t{}",
            self.maps,
            self.tiles.len(),
            self.written_tiles,
            self.empty_tiles,
            self.output_bytes,
            self.builder_work,
            self.save_work,
        )
        .expect("String formatting");
        writeln!(out, "metadata-calls\t{}", self.metadata_calls).expect("String formatting");
        if out.len() > COVERAGE_LIMIT {
            return Err("product_navigation_coverage_limit".into());
        }
        Ok(out.into_bytes())
    }

    pub fn validate(&self) -> Result<(), Error> {
        self.context.validate()?;
        let selected_tiles = self
            .identity
            .closure
            .iter()
            .find(|map| map.id == self.identity.selected_map && map.output)
            .map(|map| map.tiles)
            .ok_or("product_navigation_coverage")?;
        let completed_tiles = self
            .written_tiles
            .checked_add(self.empty_tiles)
            .ok_or("product_navigation_coverage")?;
        let observed_written_tiles = self.tiles.iter().filter(|tile| tile.written).count();
        let observed_empty_tiles = self
            .tiles
            .len()
            .checked_sub(observed_written_tiles)
            .ok_or("product_navigation_coverage")?;
        let tile_bytes = self.tiles.iter().try_fold(0_u64, |total, tile| {
            total
                .checked_add(tile.bytes)
                .ok_or_else(|| Error::from("product_navigation_coverage"))
        })?;
        let output_bytes = self
            .global_bytes
            .checked_add(tile_bytes)
            .ok_or("product_navigation_coverage")?;
        if self.context.build != 70450
            || self.context.components != Component::Navigation.bit()
            || self.context.plan_sha256 != self.identity.plan_sha256
            || self.context.producer_sha256 != self.identity.producer_sha256
            || self.context.locale.name() != self.identity.locale
            || self.identity.build != self.context.build
            || self.identity.selected_map == u32::MAX
            || selected_tiles > 4096
            || self.tiles.len() > 4096
            || self.tiles.len() != selected_tiles
            || self.maps != 1
            || completed_tiles != self.tiles.len()
            || self.written_tiles != observed_written_tiles
            || self.empty_tiles != observed_empty_tiles
            || output_bytes != self.output_bytes
        {
            return Err("product_navigation_coverage".into());
        }
        if self.identity.map.file_data_id == 0
            || self.global_bytes == 0
            || self.global_sha256 == [0; 32]
            || self.identity.map.declared_bytes == 0
            || self.identity.map.declared_bytes > MAP_LIMIT
            || self.identity.map.available_bytes > self.identity.map.declared_bytes
            || (self.identity.map.full_ckey_verified
                && self.identity.map.available_bytes != self.identity.map.declared_bytes)
            || (!self.identity.map.full_ckey_verified
                && !self.identity.map.encoded_key_authenticated)
            || self.identity.closure.is_empty()
            || self
                .identity
                .closure
                .iter()
                .filter(|map| map.output)
                .count()
                != 1
            || !self
                .identity
                .closure
                .iter()
                .any(|map| map.id == self.identity.selected_map && map.output)
            || self.identity.closure.len() > 4096
            || self.identity.terrain_maps.len() != self.identity.closure.len()
            || self.identity.map.decoded_sha256 == [0; 32]
            || self.identity.map.snapshot_sha256 == [0; 32]
            || self.identity.map.range_count == 0
            || self.identity.map.range_count > MAP_RANGES_MAX
            || self.identity.map.section_count == 0
            || self.identity.map.section_count > 16
            || self.identity.map.declared_direct_rows > MAP_ROWS_MAX
            || self.identity.map.declared_copy_pairs > MAP_ROWS_MAX
            || self.identity.map.direct_rows > self.identity.map.declared_direct_rows
            || self.identity.map.copy_pairs > self.identity.map.declared_copy_pairs
            || self.identity.map.terrain_materialized_rows
                > self
                    .identity
                    .map
                    .direct_rows
                    .checked_add(self.identity.map.copy_pairs)
                    .ok_or("product_navigation_coverage")?
            || self.identity.map.vmap_materialized_rows < self.identity.map.direct_rows
            || self.identity.map.vmap_materialized_rows
                > self
                    .identity
                    .map
                    .direct_rows
                    .checked_add(self.identity.map.copy_pairs)
                    .ok_or("product_navigation_coverage")?
            || self.identity.liquid.sha256 == [0; 32]
            || self.identity.liquid.file_data_id == 0
            || self.identity.liquid.bytes == 0
            || self.identity.liquid.direct_rows > MAP_ROWS_MAX
            || self.identity.liquid.copy_pairs > MAP_ROWS_MAX
        {
            return Err("product_navigation_coverage".into());
        }
        if self
            .tiles
            .windows(2)
            .any(|w| (w[0].tile.row, w[0].tile.column) >= (w[1].tile.row, w[1].tile.column))
            || self.tiles.iter().any(|tile| {
                tile.tile.row >= 64
                    || tile.tile.column >= 64
                    || if tile.written {
                        tile.bytes == 0 || tile.sha256 == [0; 32]
                    } else {
                        tile.bytes != 0 || tile.sha256 != [0; 32]
                    }
            })
        {
            return Err("product_navigation_coverage".into());
        }
        let closure_ids: Vec<_> = self.identity.closure.iter().map(|map| map.id).collect();
        if self.identity.closure.windows(2).any(|w| w[0].id >= w[1].id)
            || self.identity.closure.iter().any(|map| {
                map.root != self.identity.closure[0].root
                    || map.tiles > 4096
                    || (map
                        .parent
                        .or(map.cosmetic_parent)
                        .is_some_and(|parent| !closure_ids.contains(&parent)))
                    || (map.id != map.root && map.parent.is_none() && map.cosmetic_parent.is_none())
                    || (self
                        .identity
                        .closure
                        .iter()
                        .any(|child| child.parent.or(child.cosmetic_parent) == Some(map.id))
                        && !map.is_parent)
            })
        {
            return Err("product_navigation_coverage".into());
        }
        closure_depths(&self.identity.closure)?;
        let terrain_ids: Vec<_> = self
            .identity
            .terrain_maps
            .iter()
            .map(|map| map.id)
            .collect();
        if self
            .identity
            .terrain_maps
            .windows(2)
            .any(|w| w[0].id >= w[1].id)
            || terrain_ids != closure_ids
            || self.identity.terrain_maps.iter().any(|map| {
                !map.loaded
                    || map.known_present > 4096
                    || map.converted != map.known_present
                    || map.missing_root_adts != 0
            })
        {
            return Err("product_navigation_coverage".into());
        }
        let upstream_contexts = [
            (&self.identity.terrain, Component::Terrain.bit()),
            (&self.identity.vmap, Component::Collision.bit()),
        ];
        for (package, components) in upstream_contexts {
            package.context.validate()?;
            if package.files == 0
                || package.bytes == 0
                || package.context.build != 70450
                || package.context.locale.name() != self.identity.locale
                || package.context.components != components
                || package.context.source_sha256 != self.identity.current_source_sha256
                || package.context.plan_sha256 != package.plan_sha256
                || package.inventory_sha256 == [0; 32]
                || package.coverage_sha256 == [0; 32]
            {
                return Err("product_navigation_coverage".into());
            }
        }
        let map_id = catalog::resolve_for(catalog::WOWDBDEFS_70450_CATALOG_ID, "Map")
            .map_err(|_| Error::from("product_navigation_identity"))?
            .ok_or("product_navigation_identity")?
            .metadata()
            .file_data_id;
        let liquid_type_id =
            catalog::resolve_for(catalog::WOWDBDEFS_70450_CATALOG_ID, "LiquidType")
                .map_err(|_| Error::from("product_navigation_identity"))?
                .ok_or("product_navigation_identity")?
                .metadata()
                .file_data_id;
        if self.identity.map.file_data_id != map_id
            || self.identity.liquid.file_data_id != liquid_type_id
        {
            return Err("product_navigation_coverage".into());
        }
        let _ = self.identity.render()?;
        Ok(())
    }

    pub fn parse(bytes: &[u8]) -> Result<Self, Error> {
        if bytes.len() > COVERAGE_LIMIT || !bytes.is_ascii() {
            return Err("product_navigation_coverage".into());
        }
        let text =
            std::str::from_utf8(bytes).map_err(|_| Error::from("product_navigation_coverage"))?;
        let mut lines = text.lines();
        require_fields(&mut lines, "navigation-coverage", &["1"])?;
        require_fields(&mut lines, "profile", &[OUTPUT_PROFILE])?;
        let context = Context::parse(required_fields(&mut lines, "context", 1)?[0])?;
        let identity_digest = parse_hex32(required_fields(&mut lines, "identity-sha256", 1)?[0])?;
        let identity_bytes = parse_hex_bytes(required_fields(&mut lines, "identity-hex", 1)?[0])?;
        if integrity::digest(&identity_bytes) != identity_digest {
            return Err("product_navigation_identity".into());
        }
        let identity = SourceIdentity::parse(&identity_bytes)?;
        let global = required_fields(&mut lines, "global", 3)?;
        let global_map = u32_number(global[0])?;
        let global_bytes = number(global[1])?;
        let global_sha256 = parse_hex32(global[2])?;
        if global_map != identity.selected_map {
            return Err("product_navigation_coverage".into());
        }
        let mut tiles = Vec::new();
        let mut finish = None;
        let mut metadata_calls = None;
        for line in lines {
            if line.starts_with("tile\t") {
                if finish.is_some() || tiles.len() == 4096 {
                    return Err("product_navigation_coverage".into());
                }
                let values: Vec<_> = line.split('\t').collect();
                if values.len() != 6 || values[0] != "tile" {
                    return Err("product_navigation_coverage".into());
                }
                let row = u8::try_from(number(values[1])?)
                    .map_err(|_| Error::from("product_navigation_coverage"))?;
                let column = u8::try_from(number(values[2])?)
                    .map_err(|_| Error::from("product_navigation_coverage"))?;
                let written = match values[3] {
                    "written" => true,
                    "empty" => false,
                    _ => return Err("product_navigation_coverage".into()),
                };
                tiles.push(TileOutput {
                    tile: Coordinate { row, column },
                    written,
                    bytes: number(values[4])?,
                    sha256: parse_hex32(values[5])?,
                });
            } else if line.starts_with("finish\t") {
                if finish.is_some() {
                    return Err("product_navigation_coverage".into());
                }
                let values: Vec<_> = line.split('\t').collect();
                if values.len() != 8 || values[0] != "finish" {
                    return Err("product_navigation_coverage".into());
                }
                finish = Some([
                    number(values[1])?,
                    number(values[2])?,
                    number(values[3])?,
                    number(values[4])?,
                    number(values[5])?,
                    number(values[6])?,
                    number(values[7])?,
                ]);
            } else if line.starts_with("metadata-calls\t") {
                if metadata_calls.is_some() {
                    return Err("product_navigation_coverage".into());
                }
                metadata_calls = Some(number(
                    required_fields(&mut std::iter::once(line), "metadata-calls", 1)?[0],
                )?);
            } else {
                return Err("product_navigation_coverage".into());
            }
        }
        let values = finish.ok_or("product_navigation_coverage")?;
        let to_usize =
            |n| usize::try_from(n).map_err(|_| Error::from("product_navigation_coverage"));
        let coverage = Self {
            context,
            identity,
            global_bytes,
            global_sha256,
            written_tiles: to_usize(values[2])?,
            empty_tiles: to_usize(values[3])?,
            maps: to_usize(values[0])?,
            output_bytes: values[4],
            builder_work: values[5],
            save_work: values[6],
            metadata_calls: metadata_calls.ok_or("product_navigation_coverage")?,
            tiles,
        };
        coverage.validate()?;
        if coverage.render()?.as_slice() != bytes {
            return Err("product_navigation_coverage".into());
        }
        Ok(coverage)
    }
}

impl SourceIdentity {
    pub fn parse(bytes: &[u8]) -> Result<Self, Error> {
        let text =
            std::str::from_utf8(bytes).map_err(|_| Error::from("product_navigation_identity"))?;
        let mut lines = text.lines();
        require_fields(&mut lines, "identity", &["2"])?;
        require_fields(&mut lines, "profile", &[IDENTITY_DOMAIN])?;
        let selected_map = u32_number(required_fields(&mut lines, "selected-map", 1)?[0])?;
        let build = u32_number(required_fields(&mut lines, "build", 1)?[0])?;
        let locale = required_fields(&mut lines, "locale", 1)?[0].to_owned();
        let plan_sha256 = parse_hex32(required_fields(&mut lines, "plan-sha256", 1)?[0])?;
        let producer_sha256 = parse_hex32(required_fields(&mut lines, "producer-sha256", 1)?[0])?;
        let current_source_sha256 =
            parse_hex32(required_fields(&mut lines, "current-source-sha256", 1)?[0])?;
        let map_fields = required_fields(&mut lines, "map-snapshot", 15)?;
        let map = MapSnapshot {
            file_data_id: u32_number(map_fields[0])?,
            declared_bytes: number(map_fields[1])?,
            available_bytes: number(map_fields[2])?,
            full_ckey_verified: boolean(map_fields[3])?,
            encoded_key_authenticated: boolean(map_fields[4])?,
            range_count: usize_number(map_fields[5])?,
            section_count: usize_number(map_fields[6])?,
            declared_direct_rows: usize_number(map_fields[7])?,
            declared_copy_pairs: usize_number(map_fields[8])?,
            direct_rows: usize_number(map_fields[9])?,
            copy_pairs: usize_number(map_fields[10])?,
            terrain_materialized_rows: usize_number(map_fields[11])?,
            vmap_materialized_rows: usize_number(map_fields[12])?,
            decoded_sha256: parse_hex32(map_fields[13])?,
            snapshot_sha256: parse_hex32(map_fields[14])?,
        };
        let mut closure = Vec::new();
        while lines
            .clone()
            .next()
            .is_some_and(|line| line.starts_with("closure\t"))
        {
            if closure.len() == 4096 {
                return Err("product_navigation_identity".into());
            }
            let fields = required_fields(&mut lines, "closure", 8)?;
            closure.push(ClosureMap {
                id: u32_number(fields[0])?,
                root: u32_number(fields[1])?,
                parent: optional_u32(fields[2])?,
                cosmetic_parent: optional_u32(fields[3])?,
                wdt_file_data_id: u32_number(fields[4])?,
                output: boolean(fields[5])?,
                is_parent: boolean(fields[6])?,
                tiles: usize_number(fields[7])?,
            });
        }
        let liquid_fields = required_fields(&mut lines, "liquid-type", 5)?;
        let liquid = LiquidSnapshot {
            file_data_id: u32_number(liquid_fields[0])?,
            bytes: number(liquid_fields[1])?,
            direct_rows: usize_number(liquid_fields[2])?,
            copy_pairs: usize_number(liquid_fields[3])?,
            sha256: parse_hex32(liquid_fields[4])?,
        };
        let terrain = read_package_identity(&mut lines, "terrain")?;
        let mut terrain_maps = Vec::new();
        while lines
            .clone()
            .next()
            .is_some_and(|line| line.starts_with("terrain-map\t"))
        {
            if terrain_maps.len() == 4096 {
                return Err("product_navigation_identity".into());
            }
            let fields = required_fields(&mut lines, "terrain-map", 5)?;
            terrain_maps.push(TerrainMap {
                id: u32_number(fields[0])?,
                loaded: boolean(fields[1])?,
                known_present: usize_number(fields[2])?,
                converted: usize_number(fields[3])?,
                missing_root_adts: usize_number(fields[4])?,
            });
        }
        let vmap = read_package_identity(&mut lines, "vmap")?;
        if lines.next().is_some()
            || (locale != "enUS" && locale != "esES")
            || build != 70450
            || closure.windows(2).any(|w| w[0].id >= w[1].id)
            || terrain_maps.windows(2).any(|w| w[0].id >= w[1].id)
        {
            return Err("product_navigation_identity".into());
        }
        let parsed = Self {
            selected_map,
            build,
            locale,
            plan_sha256,
            producer_sha256,
            current_source_sha256,
            map,
            closure,
            liquid,
            terrain,
            terrain_maps,
            vmap,
        };
        if parsed.render()?.as_bytes() != bytes {
            return Err("product_navigation_identity".into());
        }
        Ok(parsed)
    }
}

fn read_package_identity<'a>(
    lines: &mut impl Iterator<Item = &'a str>,
    name: &str,
) -> Result<PackageIdentity, Error> {
    let context = Context::parse(required_fields(lines, &format!("{name}-context"), 1)?[0])?;
    let plan_sha256 = parse_hex32(required_fields(lines, &format!("{name}-plan-sha256"), 1)?[0])?;
    let fields = required_fields(lines, &format!("{name}-package"), 4)?;
    Ok(PackageIdentity {
        context,
        plan_sha256,
        files: number(fields[0])?,
        bytes: number(fields[1])?,
        inventory_sha256: parse_hex32(fields[2])?,
        coverage_sha256: parse_hex32(fields[3])?,
    })
}

fn required_fields<'a>(
    lines: &mut impl Iterator<Item = &'a str>,
    name: &str,
    count: usize,
) -> Result<Vec<&'a str>, Error> {
    let line = lines.next().ok_or("product_navigation_coverage")?;
    let fields: Vec<_> = line.split('\t').collect();
    if fields.len() != count + 1 || fields[0] != name || fields[1..].iter().any(|s| s.is_empty()) {
        return Err("product_navigation_coverage".into());
    }
    Ok(fields[1..].to_vec())
}

fn require_fields<'a>(
    lines: &mut impl Iterator<Item = &'a str>,
    name: &str,
    expected: &[&str],
) -> Result<(), Error> {
    let fields = required_fields(lines, name, expected.len())?;
    if fields.as_slice() != expected {
        return Err("product_navigation_coverage".into());
    }
    Ok(())
}

fn number(s: &str) -> Result<u64, Error> {
    let value = s
        .parse::<u64>()
        .map_err(|_| Error::from("product_navigation_coverage"))?;
    if value.to_string() != s {
        return Err("product_navigation_coverage".into());
    }
    Ok(value)
}
fn usize_number(s: &str) -> Result<usize, Error> {
    usize::try_from(number(s)?).map_err(|_| Error::from("product_navigation_coverage"))
}
fn u32_number(s: &str) -> Result<u32, Error> {
    u32::try_from(number(s)?).map_err(|_| Error::from("product_navigation_coverage"))
}
fn boolean(s: &str) -> Result<bool, Error> {
    match s {
        "true" => Ok(true),
        "false" => Ok(false),
        _ => Err("product_navigation_coverage".into()),
    }
}
fn optional_u32(s: &str) -> Result<Option<u32>, Error> {
    if s == "none" {
        Ok(None)
    } else {
        Ok(Some(u32_number(s)?))
    }
}
fn parse_hex32(s: &str) -> Result<[u8; 32], Error> {
    let bytes = parse_hex_bytes(s)?;
    bytes
        .try_into()
        .map_err(|_| Error::from("product_navigation_coverage"))
}
fn parse_hex_bytes(s: &str) -> Result<Vec<u8>, Error> {
    if !s.len().is_multiple_of(2)
        || !s
            .bytes()
            .all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase())
    {
        return Err("product_navigation_coverage".into());
    }
    let mut out = Vec::new();
    out.try_reserve_exact(s.len() / 2)
        .map_err(|_| Error::from("product_allocation"))?;
    for pair in s.as_bytes().as_chunks::<2>().0 {
        let digit = |b: u8| match b {
            b'0'..=b'9' => b - b'0',
            b'a'..=b'f' => b - b'a' + 10,
            _ => 255,
        };
        let hi = digit(pair[0]);
        let lo = digit(pair[1]);
        if hi > 15 || lo > 15 {
            return Err("product_navigation_coverage".into());
        }
        out.push((hi << 4) | lo);
    }
    Ok(out)
}
fn hex_bytes(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut out = String::with_capacity(bytes.len() * 2);
    for &byte in bytes {
        out.push(HEX[(byte >> 4) as usize] as char);
        out.push(HEX[(byte & 15) as usize] as char);
    }
    out
}

pub fn closure_depths(closure: &[ClosureMap]) -> Result<BTreeMap<u32, u32>, Error> {
    let parents: BTreeMap<_, _> = closure
        .iter()
        .map(|entry| (entry.id, entry.parent.or(entry.cosmetic_parent)))
        .collect();
    if parents.len() != closure.len() {
        return Err("product_navigation_closure".into());
    }
    let root = closure.first().ok_or("product_navigation_closure")?.root;
    let root_entry = closure
        .iter()
        .find(|entry| entry.id == root)
        .ok_or("product_navigation_closure")?;
    if root_entry.parent.or(root_entry.cosmetic_parent).is_some() {
        return Err("product_navigation_closure".into());
    }
    let mut depths = BTreeMap::new();
    for entry in closure {
        let mut current = entry.id;
        let mut depth = 0_u32;
        for _ in 0..=closure.len() {
            if current == root {
                break;
            }
            current = parents
                .get(&current)
                .copied()
                .flatten()
                .filter(|parent| parents.contains_key(parent))
                .ok_or("product_navigation_closure")?;
            depth = depth.checked_add(1).ok_or("product_navigation_closure")?;
        }
        if current != root {
            return Err("product_navigation_closure".into());
        }
        depths.insert(entry.id, depth);
    }
    Ok(depths)
}
