// SPDX-License-Identifier: GPL-3.0-or-later
// Added 2026-10-20: selected-navigation package admission implementation.
//! Offline verification for the v23 selected-navigation envelope.

use super::{COVERAGE_LIMIT, COVERAGE_PATH, Coverage, Error};
use crate::{mmap, tilebuilder::Coordinate};
use forever_casc::{
    Locale, integrity,
    package::{self, Artifact, Component, Context, Summary},
};
use std::{
    cell::Cell,
    collections::{BTreeMap, BTreeSet},
    fs,
    fs::File,
    io::{Read, Seek, SeekFrom},
    path::{Component as PathComponent, Path, PathBuf},
};

/// Inputs that a consumer derives from its already validated v23 product plan.
/// The values are comparison metadata, not an authority token.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Expected {
    pub selected_map: u32,
    pub locale: Locale,
    pub plan_sha256: [u8; 32],
    pub context: Option<Context>,
}

/// Downward-only ceilings for reading an admitted global and requested tiles.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ReadLimits {
    /// Maximum number of distinct candidate tile coordinates in one request.
    pub max_requested_tiles: usize,
    /// Maximum combined bytes retained for the global and requested tiles.
    pub max_batch_bytes: usize,
    /// Maximum bytes read from any one MMap artifact during batch admission.
    pub max_file_bytes: usize,
}

impl Default for ReadLimits {
    fn default() -> Self {
        Self {
            max_requested_tiles: 64,
            max_batch_bytes: 128 * 1024 * 1024,
            max_file_bytes: mmap::Limits::default().file_bytes,
        }
    }
}

impl ReadLimits {
    fn validate(self) -> Result<(), Error> {
        let defaults = Self::default();
        if self.max_requested_tiles > defaults.max_requested_tiles
            || self.max_batch_bytes > defaults.max_batch_bytes
            || self.max_file_bytes > defaults.max_file_bytes
        {
            return Err("product_navigation_read_limit".into());
        }
        Ok(())
    }
}

/// A verified package summary and its immutable parsed coverage view.
///
/// The result describes the package at verification time. It does not pin the
/// files after return; callers must independently reverify before later loads.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct VerifiedNavigation {
    summary: Summary,
    coverage: Coverage,
}

impl VerifiedNavigation {
    /// Returns the common package integrity summary from this verification.
    pub const fn summary(&self) -> &Summary {
        &self.summary
    }

    /// Returns the canonical selected-navigation coverage parsed at admission.
    pub const fn coverage(&self) -> &Coverage {
        &self.coverage
    }

    /// Consumes the admission result and returns its verified summary and
    /// canonical coverage without cloning the coverage allocation.
    pub fn into_parts(self) -> (Summary, Coverage) {
        (self.summary, self.coverage)
    }
}

/// A caller-requested tile outcome. `None` from `bytes()` means the coverage
/// explicitly records this candidate tile as empty.
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
    /// Returns the candidate tile coordinate associated with this outcome.
    pub const fn tile(&self) -> Coordinate {
        self.tile
    }

    /// Returns the verified container when written, or `None` for declared empty.
    pub fn bytes(&self) -> Option<&[u8]> {
        self.bytes.as_deref()
    }

    /// Returns whether coverage declared this candidate tile empty.
    pub const fn is_empty(&self) -> bool {
        self.bytes.is_none()
    }
}

/// Verified metadata and only the selected global plus explicitly requested
/// tile containers, retained from the same package verifier descriptor pass.
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
    /// Returns the common package summary from the successful admission pass.
    pub const fn summary(&self) -> &Summary {
        self.verified.summary()
    }

    /// Returns the canonical coverage parsed during the same admission pass.
    pub const fn coverage(&self) -> &Coverage {
        self.verified.coverage()
    }

    /// Returns the verified selected global container bytes.
    pub fn global_bytes(&self) -> &[u8] {
        &self.global
    }

    /// Results preserve the request order. Every requested coordinate is
    /// either a verified written tile or a coverage-declared empty outcome.
    pub fn tiles(&self) -> &[RequestedTile] {
        &self.tiles
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

/// Bounded immutable request metadata used to budget the selected inputs from
/// the complete manifest inventory before the package engine hashes payloads.
struct ReadAdmissionPlan {
    global_path: String,
    requested_paths: Vec<String>,
    limits: ReadLimits,
}

impl ReadState {
    fn new(selected_map: u32, requested: &[Coordinate], limits: ReadLimits) -> Result<Self, Error> {
        limits.validate()?;
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
        let mut retained = 0usize;
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

/// Verifies a Linux v23 selected-navigation package without client reads.
///
/// Admission checks the common rooted package, exact selected-map inventory,
/// MMap framing and canonical stored provenance. Coverage remains
/// producer-reported and self-consistent; this operation makes no topology,
/// source-authentication, full-world, complete-geometry or runtime claim.
pub fn verify(root: &Path, expected: Expected) -> Result<VerifiedNavigation, Error> {
    verify_inner(root, expected, None).map(|(verified, _)| verified)
}

/// Verifies a package and returns only the selected global plus explicitly
/// requested tile bytes from the same rooted descriptor pass. Empty results
/// are returned only when coverage explicitly records the requested candidate
/// as empty; requests outside coverage fail.
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
    if global.len() as u64 != verified.coverage.global_bytes
        || integrity::digest(&global) != verified.coverage.global_sha256
    {
        return Err("product_input_changed".into());
    }
    let mut tiles = Vec::new();
    tiles
        .try_reserve_exact(state.requested.len())
        .map_err(|_| Error::from("product_allocation"))?;
    for (index, &coordinate) in state.requested.iter().enumerate() {
        let fact = verified
            .coverage
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
            if artifacts.len() > 4098
                || artifacts.is_empty()
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
                let global_path = mmap::global_path(selected);
                let retain_global = read_state.is_some() && artifact.path == global_path;
                let requested_index = read_state
                    .as_ref()
                    .and_then(|state| state.path_indices.get(&artifact.path).copied());
                let retain_artifact = retain_global || requested_index.is_some();
                if retain_artifact {
                    let state = read_state.as_ref().expect("read state was checked");
                    state.check_additional(artifact.bytes)?;
                }
                let max = if artifact.path == COVERAGE_PATH {
                    COVERAGE_LIMIT
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
                    let parsed = Coverage::parse(&bytes)?;
                    if parsed.identity.selected_map != selected {
                        return Err("product_navigation_coverage".into());
                    }
                    coverage_file = Some((artifact.bytes, artifact.sha256));
                    coverage = Some(parsed);
                } else if artifact.path == global_path {
                    {
                        let view =
                            mmap::GlobalView::parse(&bytes, selected, mmap::Limits::default())
                                .map_err(|error| Error::from(error.code()))?;
                        if view.sources().len() != 0 {
                            return Err("product_navigation_offmesh".into());
                        }
                    }
                    if let Some(state) = read_state.as_mut().filter(|_| retain_global) {
                        // Successful `read_bounded` buffers have exact capacity;
                        // charge the retained allocation, not just file length.
                        state.retain_capacity(bytes.capacity())?;
                        state.global = Some(bytes);
                    }
                } else {
                    let _tile = parse_mmap_tile_path(&artifact.path, selected)?;
                    {
                        let _view = mmap::TileView::parse(&bytes, mmap::Limits::default())
                            .map_err(|error| Error::from(error.code()))?;
                    }
                    if let (Some(state), Some(index)) = (read_state.as_mut(), requested_index) {
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

    let coverage = coverage.ok_or("product_navigation_coverage")?;
    let coverage_fact = coverage_file.ok_or("product_navigation_coverage")?;
    let actual_artifacts = admitted.ok_or("product_navigation_coverage")?;
    verify_output_inventory(&actual_artifacts, &coverage, coverage_fact)?;
    if summary.context != coverage.context
        || summary.context.build != 70450
        || summary.context.locale != expected.locale
        || summary.context.components != Component::Navigation.bit()
        || summary.context.plan_sha256 != expected.plan_sha256
        || summary.context.producer_sha256 != coverage.identity.producer_sha256
        || coverage.identity.plan_sha256 != expected.plan_sha256
        || coverage.identity.build != 70450
        || coverage.identity.locale != expected.locale.name()
        || coverage.identity.selected_map != selected
        || summary.files != actual_artifacts.len()
    {
        return Err("product_navigation_identity".into());
    }
    Ok((VerifiedNavigation { summary, coverage }, read_state))
}

fn offline_output_limits() -> package::Limits {
    package::Limits {
        total_bytes: 1024 * 1024 * 1024 * 1024,
        ..package::Limits::default()
    }
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

fn parse_mmap_tile_path(path: &str, selected: u32) -> Result<Coordinate, Error> {
    let prefix = format!("mmaps/{selected:04}_");
    let suffix = ".mmtile";
    let body = path
        .strip_prefix(&prefix)
        .and_then(|value| value.strip_suffix(suffix))
        .ok_or("product_navigation_output_inventory")?;
    let (row, column) = body
        .split_once('_')
        .ok_or("product_navigation_output_inventory")?;
    if row.len() != 2 || column.len() != 2 || column.contains('_') {
        return Err("product_navigation_output_inventory".into());
    }
    let coordinate = Coordinate {
        row: row
            .parse::<u8>()
            .map_err(|_| Error::from("product_navigation_output_inventory"))?,
        column: column
            .parse::<u8>()
            .map_err(|_| Error::from("product_navigation_output_inventory"))?,
    };
    if mmap::tile_path(selected, coordinate.row, coordinate.column)
        .map_err(|error| Error::from(error.code()))?
        != path
    {
        return Err("product_navigation_output_inventory".into());
    }
    Ok(coordinate)
}

fn verify_output_inventory(
    actual: &[Artifact],
    coverage: &Coverage,
    coverage_file: (u64, [u8; 32]),
) -> Result<(), Error> {
    let expected_len = 2usize
        .checked_add(coverage.written_tiles)
        .ok_or("product_navigation_output_inventory")?;
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
    expected.push(Artifact {
        component: Component::Navigation,
        path: mmap::global_path(coverage.identity.selected_map),
        bytes: coverage.global_bytes,
        sha256: coverage.global_sha256,
    });
    for tile in &coverage.tiles {
        if tile.written {
            expected.push(Artifact {
                component: Component::Navigation,
                path: mmap::tile_path(
                    coverage.identity.selected_map,
                    tile.tile.row,
                    tile.tile.column,
                )
                .map_err(|error| Error::from(error.code()))?,
                bytes: tile.bytes,
                sha256: tile.sha256,
            });
        }
    }
    expected.sort_unstable_by(|left, right| left.path.cmp(&right.path));
    if actual != expected {
        return Err("product_navigation_output_inventory".into());
    }
    for artifact in actual {
        if artifact.path != COVERAGE_PATH
            && artifact.path != mmap::global_path(coverage.identity.selected_map)
            && !coverage.tiles.iter().any(|tile| {
                tile.written
                    && mmap::tile_path(
                        coverage.identity.selected_map,
                        tile.tile.row,
                        tile.tile.column,
                    )
                    .is_ok_and(|path| path == artifact.path)
            })
        {
            return Err("product_navigation_output_inventory".into());
        }
    }
    Ok(())
}
