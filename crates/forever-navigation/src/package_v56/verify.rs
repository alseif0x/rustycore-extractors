// SPDX-License-Identifier: GPL-3.0-or-later
// Added 2026-10-31: verified selected-map reads from complete v56 packages.

use super::{COVERAGE_MAX, COVERAGE_PATH, Coverage56, Error, MapFact, Output, ReadLimits};
use crate::{mmap, tilebuilder::Coordinate};
use forever_casc::{
    Locale, integrity,
    package::{self, Artifact, Component, Context, Summary},
};
use std::{
    cell::Cell,
    collections::{BTreeMap, BTreeSet},
    fs::{self, File},
    io::{Read, Seek, SeekFrom},
    path::{Component as PathComponent, Path, PathBuf},
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Expected {
    pub selected_map: u32,
    pub locale: Locale,
    pub plan_sha256: [u8; 32],
    pub context: Option<Context>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct VerifiedNavigation {
    summary: Summary,
    coverage: Coverage56,
    selected: usize,
}

impl VerifiedNavigation {
    pub const fn summary(&self) -> &Summary {
        &self.summary
    }

    pub const fn coverage(&self) -> &Coverage56 {
        &self.coverage
    }

    pub fn map(&self) -> &MapFact {
        &self.coverage.maps[self.selected]
    }

    pub fn output(&self) -> &Output {
        &self.coverage.outputs[self.selected]
    }
}

#[derive(PartialEq, Eq)]
pub struct RequestedTile {
    tile: Coordinate,
    bytes: Option<Vec<u8>>,
}

impl std::fmt::Debug for RequestedTile {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("RequestedTile")
            .field("tile", &self.tile)
            .field("written_bytes", &self.bytes.as_ref().map(Vec::len))
            .finish()
    }
}

impl RequestedTile {
    pub const fn tile(&self) -> Coordinate {
        self.tile
    }

    pub fn bytes(&self) -> Option<&[u8]> {
        self.bytes.as_deref()
    }

    pub const fn is_empty(&self) -> bool {
        self.bytes.is_none()
    }
}

#[derive(PartialEq, Eq)]
pub struct ReadBatch {
    verified: VerifiedNavigation,
    global: Vec<u8>,
    tiles: Vec<RequestedTile>,
}

impl std::fmt::Debug for ReadBatch {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("ReadBatch")
            .field("summary", self.verified.summary())
            .field("global_bytes", &self.global.len())
            .field("tiles", &self.tiles)
            .finish()
    }
}

impl ReadBatch {
    pub const fn summary(&self) -> &Summary {
        self.verified.summary()
    }

    pub const fn coverage(&self) -> &Coverage56 {
        self.verified.coverage()
    }

    pub fn map(&self) -> &MapFact {
        self.verified.map()
    }

    pub fn output(&self) -> &Output {
        self.verified.output()
    }

    pub fn global_bytes(&self) -> &[u8] {
        &self.global
    }

    pub fn tiles(&self) -> &[RequestedTile] {
        &self.tiles
    }

    pub fn into_byte_parts(self) -> super::ByteParts {
        let tiles = self
            .tiles
            .into_iter()
            .map(|requested| (requested.tile, requested.bytes))
            .collect();
        (self.global, tiles)
    }
}

struct ReadState {
    limits: ReadLimits,
    requested: Vec<Coordinate>,
    path_indices: BTreeMap<String, usize>,
    global: Option<Vec<u8>>,
    tile_bytes: Vec<Option<Vec<u8>>>,
    retained_capacity: usize,
}

struct ReadAdmissionPlan {
    global_path: String,
    requested_paths: Vec<String>,
    limits: ReadLimits,
}

impl ReadState {
    fn new(selected_map: u32, requested: &[Coordinate], limits: ReadLimits) -> Result<Self, Error> {
        let defaults = ReadLimits::default();
        if limits.max_requested_tiles > defaults.max_requested_tiles
            || limits.max_batch_bytes > defaults.max_batch_bytes
            || limits.max_file_bytes > defaults.max_file_bytes
        {
            return Err("product_navigation_read_limit".into());
        }
        if requested.len() > limits.max_requested_tiles {
            return Err("product_navigation_read_limit".into());
        }
        let mut seen = BTreeSet::new();
        let mut owned = Vec::new();
        let mut tile_bytes = Vec::new();
        owned
            .try_reserve_exact(requested.len())
            .map_err(|_| Error::from("product_allocation"))?;
        tile_bytes
            .try_reserve_exact(requested.len())
            .map_err(|_| Error::from("product_allocation"))?;
        let mut path_indices = BTreeMap::new();
        for (index, &tile) in requested.iter().enumerate() {
            if tile.row >= 64 || tile.column >= 64 {
                return Err("product_navigation_read_request".into());
            }
            if !seen.insert((tile.row, tile.column)) {
                return Err("product_navigation_read_request".into());
            }
            let path = mmap::tile_path(selected_map, tile.row, tile.column)
                .map_err(|_| Error::from("product_navigation_read_request"))?;
            path_indices.insert(path, index);
            owned.push(tile);
            tile_bytes.push(None);
        }
        Ok(Self {
            limits,
            requested: owned,
            path_indices,
            global: None,
            tile_bytes,
            retained_capacity: 0,
        })
    }

    fn admission_plan(&self, selected_map: u32) -> Result<ReadAdmissionPlan, Error> {
        let mut requested_paths = Vec::new();
        requested_paths
            .try_reserve_exact(self.path_indices.len())
            .map_err(|_| Error::from("product_allocation"))?;
        requested_paths.extend(self.path_indices.keys().cloned());
        Ok(ReadAdmissionPlan {
            global_path: mmap::global_path(selected_map),
            requested_paths,
            limits: self.limits,
        })
    }

    fn check_additional(&self, file_bytes: u64) -> Result<(), Error> {
        let file_bytes = usize::try_from(file_bytes)
            .map_err(|_| Error::from("product_navigation_read_limit"))?;
        if file_bytes > self.limits.max_file_bytes {
            return Err("product_navigation_read_limit".into());
        }
        self.retained_capacity
            .checked_add(file_bytes)
            .filter(|total| *total <= self.limits.max_batch_bytes)
            .ok_or("product_navigation_read_limit")?;
        Ok(())
    }

    fn retain_capacity(&mut self, capacity: usize) -> Result<(), Error> {
        let total = self
            .retained_capacity
            .checked_add(capacity)
            .filter(|total| *total <= self.limits.max_batch_bytes)
            .ok_or("product_navigation_read_limit")?;
        self.retained_capacity = total;
        Ok(())
    }
}

impl ReadAdmissionPlan {
    fn check_inventory_budget(&self, artifacts: &[Artifact]) -> Result<(), Error> {
        let mut retained = 0_usize;
        for artifact in artifacts {
            if artifact.path == self.global_path
                || self
                    .requested_paths
                    .iter()
                    .any(|path| path == &artifact.path)
            {
                let bytes = usize::try_from(artifact.bytes)
                    .map_err(|_| Error::from("product_navigation_read_limit"))?;
                if bytes > self.limits.max_file_bytes {
                    return Err("product_navigation_read_limit".into());
                }
                retained = retained
                    .checked_add(bytes)
                    .filter(|total| *total <= self.limits.max_batch_bytes)
                    .ok_or("product_navigation_read_limit")?;
            }
        }
        Ok(())
    }
}

pub fn verify(root: &Path, expected: Expected) -> Result<VerifiedNavigation, Error> {
    verify_inner(root, expected, None).map(|(verified, _)| verified)
}

pub fn read_batch(
    root: &Path,
    expected: Expected,
    requested_tiles: &[Coordinate],
    limits: ReadLimits,
) -> Result<ReadBatch, Error> {
    let state = ReadState::new(expected.selected_map, requested_tiles, limits)?;
    let (verified, state) = verify_inner(root, expected, Some(state))?;
    let mut state = state.ok_or("product_navigation_read_unavailable")?;
    let global = state
        .global
        .take()
        .ok_or("product_navigation_read_unavailable")?;
    let global_fact = verified
        .output()
        .global
        .ok_or("product_navigation_read_unavailable")?;
    if global.len() as u64 != global_fact.bytes || integrity::digest(&global) != global_fact.sha256
    {
        return Err("product_input_changed".into());
    }
    let mut tiles = Vec::new();
    tiles
        .try_reserve_exact(state.requested.len())
        .map_err(|_| Error::from("product_allocation"))?;
    for (index, &coordinate) in state.requested.iter().enumerate() {
        let fact = verified
            .output()
            .tiles
            .iter()
            .find(|fact| fact.tile == coordinate)
            .ok_or("product_navigation_read_unavailable")?;
        let bytes = state.tile_bytes[index].take();
        if fact.written {
            let payload = bytes.ok_or("product_navigation_read_unavailable")?;
            if payload.len() as u64 != fact.bytes || integrity::digest(&payload) != fact.sha256 {
                return Err("product_input_changed".into());
            }
            tiles.push(RequestedTile {
                tile: coordinate,
                bytes: Some(payload),
            });
        } else {
            if bytes.is_some() {
                return Err("product_navigation_output_inventory".into());
            }
            tiles.push(RequestedTile {
                tile: coordinate,
                bytes: None,
            });
        }
    }
    Ok(ReadBatch {
        verified,
        global,
        tiles,
    })
}

fn verify_inner(
    root: &Path,
    expected: Expected,
    mut read_state: Option<ReadState>,
) -> Result<(VerifiedNavigation, Option<ReadState>), Error> {
    let selected = expected.selected_map;
    let root = absolute_without_symlinks(root)?;
    let read_plan = read_state
        .as_ref()
        .map(|state| state.admission_plan(selected))
        .transpose()?;
    let mut admitted = None;
    let mut coverage = None;
    let mut coverage_file: Option<(u64, [u8; 32])> = None;
    let failure = Cell::new(None);
    let summary = package::verify_with_preflight(
        &root,
        expected.context,
        offline_output_limits(),
        |artifacts| {
            if artifacts.is_empty()
                || artifacts.len() > super::TILES_MAX + super::MAPS_MAX + 1
                || artifacts.iter().any(|artifact| {
                    artifact.component != Component::Navigation
                        || !(artifact.path == COVERAGE_PATH || artifact.path.starts_with("mmaps/"))
                })
                || artifacts
                    .iter()
                    .filter(|artifact| artifact.path == COVERAGE_PATH)
                    .count()
                    != 1
            {
                return Err(package::Error::Inventory);
            }
            if let Some(read_plan) = &read_plan
                && let Err(error) = read_plan.check_inventory_budget(artifacts)
            {
                failure.set(Some(error));
                return Err(package::Error::Schema);
            }
            let mut retained = Vec::new();
            retained
                .try_reserve_exact(artifacts.len())
                .map_err(|_| package::Error::Limit)?;
            if retained.capacity() != artifacts.len() {
                return Err(package::Error::Limit);
            }
            retained.extend(artifacts.iter().cloned());
            admitted = Some(retained);
            Ok(())
        },
        |artifact, file| {
            let result = (|| {
                let retain_global =
                    read_state.is_some() && artifact.path == mmap::global_path(selected);
                let requested_index = read_state
                    .as_ref()
                    .and_then(|state| state.path_indices.get(&artifact.path).copied());
                let retain_artifact = retain_global || requested_index.is_some();
                if retain_artifact {
                    let state = read_state.as_ref().expect("read state was checked");
                    state.check_additional(artifact.bytes)?;
                }
                let max = if artifact.path == COVERAGE_PATH {
                    COVERAGE_MAX
                } else if let Some(state) = read_state.as_ref() {
                    if artifact.bytes > state.limits.max_file_bytes as u64 {
                        return Err("product_navigation_read_limit".into());
                    }
                    state.limits.max_file_bytes
                } else {
                    mmap::Limits::default().file_bytes
                };
                let bytes = read_bounded(file, artifact.bytes, max)?;
                let digest = integrity::digest(&bytes);
                if digest != artifact.sha256 {
                    return Err("product_input_changed".into());
                }
                if artifact.path == COVERAGE_PATH {
                    let parsed = Coverage56::parse(&bytes)?;
                    coverage_file = Some((artifact.bytes, artifact.sha256));
                    coverage = Some(parsed);
                } else {
                    match parse_artifact_path(&artifact.path)? {
                        ArtifactKind::Global(map) => {
                            let view =
                                mmap::GlobalView::parse(&bytes, map, mmap::Limits::default())
                                    .map_err(|error| Error::from(error.code()))?;
                            if view.sources().len() != 0 {
                                return Err("product_navigation_offmesh".into());
                            }
                        }
                        ArtifactKind::Tile => {
                            let _view = mmap::TileView::parse(&bytes, mmap::Limits::default())
                                .map_err(|error| Error::from(error.code()))?;
                        }
                    }
                    if retain_global {
                        let state = read_state.as_mut().expect("read state was checked");
                        state.retain_capacity(bytes.capacity())?;
                        state.global = Some(bytes);
                    } else if let (Some(state), Some(index)) =
                        (read_state.as_mut(), requested_index)
                    {
                        state.retain_capacity(bytes.capacity())?;
                        state.tile_bytes[index] = Some(bytes);
                    }
                }
                Ok(())
            })();
            result.map_err(|error| {
                failure.set(Some(error));
                package::Error::Schema
            })
        },
    )
    .map_err(|error| failure.get().unwrap_or_else(|| error.into()))?;

    let coverage: Coverage56 = coverage.ok_or("product_navigation_coverage")?;
    let coverage_fact = coverage_file.ok_or("product_navigation_coverage")?;
    let actual_artifacts = admitted.ok_or("product_navigation_coverage")?;
    verify_output_inventory(&actual_artifacts, &coverage, coverage_fact)?;
    let selected_index = coverage
        .maps
        .iter()
        .position(|map| map.id == selected && map.state == "planned")
        .ok_or("product_navigation_coverage")?;
    if summary.context != coverage.context
        || summary.context.build != 70450
        || summary.context.locale != expected.locale
        || summary.context.components != Component::Navigation.bit()
        || summary.context.plan_sha256 != expected.plan_sha256
        || summary.files != actual_artifacts.len()
    {
        return Err("product_navigation_identity".into());
    }
    Ok((
        VerifiedNavigation {
            summary,
            coverage,
            selected: selected_index,
        },
        read_state,
    ))
}

fn offline_output_limits() -> package::Limits {
    package::Limits {
        total_bytes: 1024 * 1024 * 1024 * 1024,
        ..package::Limits::default()
    }
}

enum ArtifactKind {
    Global(u32),
    Tile,
}

fn parse_artifact_path(path: &str) -> Result<ArtifactKind, Error> {
    let name = path
        .strip_prefix("mmaps/")
        .ok_or("product_navigation_output_inventory")?;
    if let Some(body) = name.strip_suffix(".mmap") {
        let map = parse_map_id(body)?;
        if mmap::global_path(map) != path {
            return Err("product_navigation_output_inventory".into());
        }
        return Ok(ArtifactKind::Global(map));
    }
    let body = name
        .strip_suffix(".mmtile")
        .ok_or("product_navigation_output_inventory")?;
    let (map, coordinates) = body
        .split_once('_')
        .ok_or("product_navigation_output_inventory")?;
    let map = parse_map_id(map)?;
    let (row, column) = coordinates
        .split_once('_')
        .ok_or("product_navigation_output_inventory")?;
    if row.len() != 2 || column.len() != 2 || column.contains('_') {
        return Err("product_navigation_output_inventory".into());
    }
    let tile = Coordinate {
        row: row
            .parse::<u8>()
            .map_err(|_| Error::from("product_navigation_output_inventory"))?,
        column: column
            .parse::<u8>()
            .map_err(|_| Error::from("product_navigation_output_inventory"))?,
    };
    if tile.row >= 64
        || tile.column >= 64
        || mmap::tile_path(map, tile.row, tile.column).map_err(|error| Error::from(error.code()))?
            != path
    {
        return Err("product_navigation_output_inventory".into());
    }
    Ok(ArtifactKind::Tile)
}

fn parse_map_id(text: &str) -> Result<u32, Error> {
    if text.len() != 4 || !text.bytes().all(|byte| byte.is_ascii_digit()) {
        return Err("product_navigation_output_inventory".into());
    }
    text.parse::<u32>()
        .map_err(|_| Error::from("product_navigation_output_inventory"))
}

fn verify_output_inventory(
    actual: &[Artifact],
    coverage: &Coverage56,
    coverage_file: (u64, [u8; 32]),
) -> Result<(), Error> {
    let expected_len = coverage.outputs.iter().try_fold(1_usize, |total, output| {
        total
            .checked_add(usize::from(output.global.is_some()))
            .and_then(|count| {
                count.checked_add(output.tiles.iter().filter(|tile| tile.written).count())
            })
            .ok_or_else(|| Error::from("product_navigation_output_inventory"))
    })?;
    if actual.len() != expected_len {
        return Err("product_navigation_output_inventory".into());
    }
    let mut expected = Vec::new();
    expected
        .try_reserve_exact(expected_len)
        .map_err(|_| Error::from("product_allocation"))?;
    expected.push(Artifact {
        component: Component::Navigation,
        path: COVERAGE_PATH.to_owned(),
        bytes: coverage_file.0,
        sha256: coverage_file.1,
    });
    for output in &coverage.outputs {
        if let Some(global) = output.global {
            expected.push(Artifact {
                component: Component::Navigation,
                path: mmap::global_path(output.map),
                bytes: global.bytes,
                sha256: global.sha256,
            });
        }
        for tile in &output.tiles {
            if tile.written {
                expected.push(Artifact {
                    component: Component::Navigation,
                    path: mmap::tile_path(output.map, tile.tile.row, tile.tile.column)
                        .map_err(|error| Error::from(error.code()))?,
                    bytes: tile.bytes,
                    sha256: tile.sha256,
                });
            }
        }
    }
    expected.sort_unstable_by(|left, right| left.path.cmp(&right.path));
    if actual != expected {
        return Err("product_navigation_output_inventory".into());
    }
    Ok(())
}

fn absolute_without_symlinks(path: &Path) -> Result<PathBuf, Error> {
    let absolute = if path.is_absolute() {
        path.to_path_buf()
    } else {
        std::env::current_dir()
            .map_err(|_| Error::from("product_navigation_path"))?
            .join(path)
    };
    if absolute.as_os_str().len() > 4096
        || absolute
            .components()
            .any(|component| matches!(component, PathComponent::ParentDir))
    {
        return Err("product_navigation_path".into());
    }
    let mut walk = PathBuf::from("/");
    let mut components = absolute
        .components()
        .filter(|part| !matches!(part, PathComponent::RootDir | PathComponent::CurDir))
        .peekable();
    while let Some(component) = components.next() {
        walk.push(component);
        let metadata =
            fs::symlink_metadata(&walk).map_err(|_| Error::from("product_navigation_path"))?;
        if metadata.file_type().is_symlink() {
            return Err("product_navigation_path".into());
        }
        if components.peek().is_some() && !metadata.is_dir() {
            return Err("product_navigation_path".into());
        }
    }
    let metadata =
        fs::symlink_metadata(&walk).map_err(|_| Error::from("product_navigation_path"))?;
    if !metadata.is_dir() {
        return Err("product_navigation_path".into());
    }
    fs::canonicalize(&absolute).map_err(|_| Error::from("product_navigation_path"))
}

fn read_bounded(file: &mut File, bytes: u64, max: usize) -> Result<Vec<u8>, Error> {
    let length =
        usize::try_from(bytes).map_err(|_| Error::from("product_navigation_input_limit"))?;
    if length > max {
        return Err("product_navigation_input_limit".into());
    }
    let mut data = Vec::new();
    data.try_reserve_exact(length)
        .map_err(|_| Error::from("product_allocation"))?;
    if data.capacity() > length {
        return Err("product_navigation_input_limit".into());
    }
    data.resize(length, 0);
    file.seek(SeekFrom::Start(0))
        .map_err(|_| Error::from("product_input_io"))?;
    file.read_exact(&mut data)
        .map_err(|_| Error::from("product_input_io"))?;
    let mut trailing = [0_u8; 1];
    if file
        .read(&mut trailing)
        .map_err(|_| Error::from("product_input_io"))?
        != 0
    {
        return Err("product_input_changed".into());
    }
    Ok(data)
}
