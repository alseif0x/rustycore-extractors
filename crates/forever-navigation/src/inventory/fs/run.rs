// SPDX-License-Identifier: GPL-3.0-or-later
// Authored 2026-10-11: bounded sequential navigation selection execution.
//! Deterministic map/tile order, fresh per-tile source/store contexts and
//! provisional output callbacks. Only `Completed` admits the entire selection.
//! No output accumulation, publication, background threads or implicit resume.
use super::*;
use crate::{mmap, recast, tilebuilder};
use std::{cell::RefCell, fmt::Write as _};

#[cfg(test)]
mod tests;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Selection {
    #[default]
    All,
    Map(u32),
    Tile {
        map_id: u32,
        tile: Coordinate,
    },
}
impl Selection {
    fn map(self, id: u32) -> bool {
        match self {
            Self::All => true,
            Self::Map(map_id) | Self::Tile { map_id, .. } => map_id == id,
        }
    }
    fn tile(self, t: Coordinate) -> bool {
        match self {
            Self::Tile { tile, .. } => t == tile,
            _ => true,
        }
    }
}
#[derive(Clone, Copy, Debug)]
pub struct Options {
    pub selection: Selection,
    pub big_base_unit: bool,
    pub slopes: recast::Slopes,
    pub skip_liquids: bool,
}
impl Default for Options {
    fn default() -> Self {
        Self {
            selection: Selection::All,
            big_base_unit: false,
            slopes: recast::Slopes::default(),
            skip_liquids: false,
        }
    }
}
#[derive(Clone, Copy, Debug)]
pub struct Limits {
    pub maps: usize,
    pub tiles: usize,
    /// Cumulative global/tile output bytes, checked before each sink callback.
    pub output_bytes: u64,
    pub global: mmap::Limits,
    pub pipeline: pipeline::Limits,
}
impl Default for Limits {
    fn default() -> Self {
        Self {
            maps: 4096,
            tiles: 4096 * 4096,
            output_bytes: 1024 * 1024 * 1024 * 1024,
            global: Default::default(),
            pipeline: Default::default(),
        }
    }
}
impl Limits {
    fn validate(self) -> Result<(), Error> {
        let d = Self::default();
        if self.maps > d.maps || self.tiles > d.tiles || self.output_bytes > d.output_bytes {
            return Err(Error::Limit);
        }
        self.global.validate().map_err(Error::Output)?;
        self.pipeline
            .validate()
            .map_err(|e| Error::Input(super::Error::Pipeline(e)))
    }
}
/// Borrowed provisional output. The sink must retain a failed-run marker and
/// must not advertise success until `execute` returns `Completed`. All earlier
/// callbacks remain provisional on *any* error, including final verification.
pub enum Event<'a> {
    Global {
        map_id: u32,
        bytes: &'a [u8],
    },
    Tile {
        map_id: u32,
        tile: Coordinate,
        outcome: &'a pipeline::Outcome,
    },
}
pub trait Sink {
    type Error;
    fn emit(&mut self, event: Event<'_>) -> Result<(), Self::Error>;
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Progress {
    pub maps: usize,
    pub tiles: usize,
    pub written_tiles: usize,
    pub empty_tiles: usize,
    pub output_bytes: u64,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Completed {
    pub selection: Selection,
    pub progress: Progress,
    /// Discovery guard calls only; excludes fresh provider/store metadata work.
    pub metadata_calls: u64,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Error {
    Selection,
    Limit,
    Input(super::Error),
    Output(mmap::Error),
}
impl Error {
    pub const fn code(self) -> &'static str {
        match self {
            Self::Selection => "navigation_run_selection",
            Self::Limit => "navigation_run_limit",
            Self::Input(e) => e.code(),
            Self::Output(e) => e.code(),
        }
    }
}
#[derive(Debug)]
pub enum Cause<E> {
    Run(Error),
    Sink(E),
}
#[derive(Debug)]
pub struct Failure<E> {
    pub cause: Cause<E>,
    /// Successfully acknowledged provisional callbacks only.
    pub progress: Progress,
}
impl<E> Failure<E> {
    pub const fn code(&self) -> &'static str {
        match &self.cause {
            Cause::Run(e) => e.code(),
            Cause::Sink(_) => "navigation_run_sink",
        }
    }
}
fn counts(package: &Package, selection: Selection) -> Result<(usize, usize), Error> {
    if let Selection::Map(id) | Selection::Tile { map_id: id, .. } = selection {
        let map = package
            .plan
            .map(id)
            .filter(|m| m.is_output())
            .ok_or(Error::Selection)?;
        if let Selection::Tile { tile, .. } = selection
            && !map.contains(tile)
        {
            return Err(Error::Selection);
        }
    }
    let mut maps = 0usize;
    let mut tiles = 0usize;
    for map in package
        .plan
        .maps()
        .iter()
        .filter(|m| m.is_output() && selection.map(m.map_id()))
    {
        maps = maps.checked_add(1).ok_or(Error::Limit)?;
        tiles = tiles
            .checked_add(if matches!(selection, Selection::Tile { .. }) {
                1
            } else {
                map.tile_count()
            })
            .ok_or(Error::Limit)?;
    }
    Ok((maps, tiles))
}
fn bytes(progress: Progress, n: usize, limits: Limits) -> Result<u64, Error> {
    let size = progress
        .output_bytes
        .checked_add(n as u64)
        .ok_or(Error::Limit)?;
    if size > limits.output_bytes {
        Err(Error::Limit)
    } else {
        Ok(size)
    }
}
/// Runs all planned output maps, one planned map, or one admitted tile. Explicit
/// selection does not override Package's discovery filters: open with its
/// `Options.map` when that override is desired. Empty All selections complete;
/// explicitly selecting an empty/ancestor-only/absent map rejects.
///
/// Global inventory verification is O(entries) at two batch boundaries. Each
/// artifact read is checked against the sorted original inventory in O(log N),
/// before and after the existing rooted provider's complete bounded read. Fresh
/// provider/store contexts bound metadata residency independently of batch size.
pub fn execute<S: Sink>(
    package: &mut Package,
    types: models::LiquidTypes<'_>,
    connections: &[offmesh::Source],
    options: Options,
    limits: Limits,
    sink: &mut S,
) -> Result<Completed, Failure<S::Error>> {
    let mut progress = Progress::default();
    let run_error = |e| Failure {
        cause: Cause::Run(e),
        progress: Progress::default(),
    };
    limits.validate().map_err(run_error)?;
    tilebuilder::Config::new(
        tilebuilder::Request {
            map_id: 0,
            tile: Coordinate { row: 0, column: 0 },
            big_base_unit: options.big_base_unit,
            slopes: options.slopes,
        },
        &crate::Mesh::default(),
    )
    .map_err(|e| {
        run_error(Error::Input(super::Error::Pipeline(
            pipeline::Error::Builder(e),
        )))
    })?;
    let (maps, tiles) = counts(package, options.selection).map_err(run_error)?;
    if maps > limits.maps || tiles > limits.tiles || connections.len() > limits.global.records {
        return Err(run_error(Error::Limit));
    }
    let calls = package.statistics().metadata_calls;
    package
        .verify_unchanged()
        .map_err(|e| run_error(Error::Input(e)))?;
    // Index traversal borrows no accumulating schedule/output Vec.
    for i in 0..package.plan.maps().len() {
        let map = &package.plan.maps()[i];
        let id = map.map_id();
        if !map.is_output() || !options.selection.map(id) {
            continue;
        }
        {
            let global = mmap::encode_global(
                id,
                map.params().ok_or_else(|| Failure {
                    cause: Cause::Run(Error::Selection),
                    progress,
                })?,
                connections,
                limits.global,
            )
            .map_err(|e| Failure {
                cause: Cause::Run(Error::Output(e)),
                progress,
            })?;
            let size = bytes(progress, global.len(), limits).map_err(|e| Failure {
                cause: Cause::Run(e),
                progress,
            })?;
            sink.emit(Event::Global {
                map_id: id,
                bytes: &global,
            })
            .map_err(|e| Failure {
                cause: Cause::Sink(e),
                progress,
            })?;
            progress.maps += 1;
            progress.output_bytes = size;
        }
        for row in 0..64 {
            for column in 0..64 {
                let tile = Coordinate { row, column };
                if !package.plan.maps()[i].contains(tile) || !options.selection.tile(tile) {
                    continue;
                }
                let request = pipeline::Request {
                    tile: tilebuilder::Request {
                        map_id: id,
                        tile,
                        big_base_unit: options.big_base_unit,
                        slopes: options.slopes,
                    },
                    skip_liquids: options.skip_liquids,
                };
                let outcome = bound_tile(package, types, connections, request, limits.pipeline)
                    .map_err(|e| Failure {
                        cause: Cause::Run(Error::Input(e)),
                        progress,
                    })?;
                let n = match &outcome {
                    pipeline::Outcome::Tile { bytes, .. } => bytes.len(),
                    _ => 0,
                };
                let size = bytes(progress, n, limits).map_err(|e| Failure {
                    cause: Cause::Run(e),
                    progress,
                })?;
                sink.emit(Event::Tile {
                    map_id: id,
                    tile,
                    outcome: &outcome,
                })
                .map_err(|e| Failure {
                    cause: Cause::Sink(e),
                    progress,
                })?;
                progress.tiles += 1;
                progress.output_bytes = size;
                match outcome {
                    pipeline::Outcome::Tile { .. } => progress.written_tiles += 1,
                    _ => progress.empty_tiles += 1,
                }
            }
        }
    }
    package.verify_unchanged().map_err(|e| Failure {
        cause: Cause::Run(Error::Input(e)),
        progress,
    })?;
    debug_assert_eq!((progress.maps, progress.tiles), (maps, tiles));
    Ok(Completed {
        selection: options.selection,
        progress,
        metadata_calls: package.statistics().metadata_calls - calls,
    })
}

struct Binding<'a> {
    guard: &'a guard::Guard,
    budget: &'a mut Budget,
}
impl Binding<'_> {
    fn check(&mut self, root: usize, name: &Name) -> Result<(), super::Error> {
        self.guard.artifact(root, name.bytes(), self.budget)
    }
}
// Stack paths, same exact producer names and closed model-name policy as the
// rooted providers. No user string can inject a directory traversal here.
struct Name {
    bytes: [u8; 511],
    len: usize,
}
impl Name {
    fn new() -> Self {
        Self {
            bytes: [0; 511],
            len: 0,
        }
    }
    fn bytes(&self) -> &[u8] {
        &self.bytes[..self.len]
    }
}
impl std::fmt::Write for Name {
    fn write_str(&mut self, s: &str) -> std::fmt::Result {
        let end = self.len.checked_add(s.len()).ok_or(std::fmt::Error)?;
        self.bytes
            .get_mut(self.len..end)
            .ok_or(std::fmt::Error)?
            .copy_from_slice(s.as_bytes());
        self.len = end;
        Ok(())
    }
}
fn terrain_name(a: terrain::load::Artifact) -> Result<Name, terrain::load::SourceError> {
    if a.map_id > 9999 || a.tile.row >= 64 || a.tile.column >= 64 {
        return Err(terrain::load::SourceError::Io);
    }
    let mut name = Name::new();
    write!(
        &mut name,
        "{:04}_{:02}_{:02}.map",
        a.map_id, a.tile.row, a.tile.column
    )
    .map_err(|_| terrain::load::SourceError::Limit)?;
    Ok(name)
}
fn collision_name(a: store::Artifact<'_>) -> Result<Name, store::SourceError> {
    let mut name = Name::new();
    match a {
        store::Artifact::Tree { map_id } if map_id <= 9999 => {
            write!(&mut name, "{map_id:04}/{map_id:04}.vmtree")
        }
        store::Artifact::Tile { map_id, x, y } | store::Artifact::Index { map_id, x, y }
            if map_id <= 9999 && x < 64 && y < 64 =>
        {
            write!(&mut name, "{map_id:04}/{map_id:04}_{x:02}_{y:02}.vmtile")
                .map_err(|_| store::SourceError::Limit)?;
            if matches!(a, store::Artifact::Index { .. }) {
                name.write_str("idx")
            } else {
                Ok(())
            }
        }
        store::Artifact::Model { name: raw }
            if !raw.is_empty()
                && raw.len() <= 500
                && raw.is_ascii()
                && raw != b"."
                && raw != b".."
                && !raw.iter().any(|&b| matches!(b, 0 | b'/' | b'\\')) =>
        {
            name.write_str(std::str::from_utf8(raw).map_err(|_| store::SourceError::Io)?)
                .map_err(|_| store::SourceError::Limit)?;
            name.write_str(".vmo")
        }
        _ => return Err(store::SourceError::Io),
    }
    .map_err(|_| store::SourceError::Limit)?;
    Ok(name)
}
fn terrain_error(e: super::Error) -> terrain::load::SourceError {
    match e {
        super::Error::Limit | super::Error::Allocation => terrain::load::SourceError::Limit,
        _ => terrain::load::SourceError::InputChanged,
    }
}
fn collision_error(e: super::Error) -> store::SourceError {
    match e {
        super::Error::Limit | super::Error::Allocation => store::SourceError::Limit,
        _ => store::SourceError::InputChanged,
    }
}
struct Terrain<'a, 'b> {
    source: terrain::load::fs::RootedSource,
    binding: &'a RefCell<Binding<'b>>,
}
impl terrain::load::Source for Terrain<'_, '_> {
    fn read(
        &mut self,
        a: terrain::load::Artifact,
        out: &mut terrain::load::ReadBuffer<'_>,
    ) -> Result<terrain::load::Presence, terrain::load::SourceError> {
        let name = terrain_name(a)?;
        self.binding
            .borrow_mut()
            .check(0, &name)
            .map_err(terrain_error)?;
        let result = self.source.read(a, out);
        if matches!(result, Err(terrain::load::SourceError::InputChanged)) {
            return result;
        }
        self.binding
            .borrow_mut()
            .check(0, &name)
            .map_err(terrain_error)?;
        result
    }
    fn verify_unchanged(&mut self) -> Result<(), terrain::load::SourceError> {
        self.source.verify_unchanged()
    }
}
struct Collision<'a, 'b> {
    source: store::fs::RootedSource,
    binding: &'a RefCell<Binding<'b>>,
}
impl store::Source for Collision<'_, '_> {
    fn read(
        &mut self,
        a: store::Artifact<'_>,
        out: &mut store::ReadBuffer<'_>,
    ) -> Result<store::Presence, store::SourceError> {
        let name = collision_name(a)?;
        self.binding
            .borrow_mut()
            .check(1, &name)
            .map_err(collision_error)?;
        let result = self.source.read(a, out);
        if matches!(result, Err(store::SourceError::InputChanged)) {
            return result;
        }
        self.binding
            .borrow_mut()
            .check(1, &name)
            .map_err(collision_error)?;
        result
    }
    fn verify_unchanged(&mut self) -> Result<(), store::SourceError> {
        self.source.verify_unchanged()
    }
}
fn bound_tile(
    p: &mut Package,
    types: models::LiquidTypes<'_>,
    connections: &[offmesh::Source],
    request: pipeline::Request,
    limits: pipeline::Limits,
) -> Result<pipeline::Outcome, super::Error> {
    if p.failed {
        return Err(super::Error::InputChanged);
    }
    let result = (|| {
        p.budget.used = 0;
        p.guard.verify_roots(&mut p.budget)?;
        let mut parents = Vec::new();
        let count = p.plan.parent_rows().count();
        p.budget
            .reserve(&mut parents, count, p.budget.limits.store.max_maps)?;
        parents.extend(p.plan.parent_rows());
        let result = (|| {
            let limits_source = p.budget.limits;
            let terrain =
                terrain::load::fs::RootedSource::new(p.guard.path(0), limits_source.terrain_source)
                    .map_err(super::Error::TerrainSource)?;
            let collision =
                store::fs::RootedSource::new(p.guard.path(1), limits_source.collision_source)
                    .map_err(super::Error::CollisionSource)?;
            let binding = RefCell::new(Binding {
                guard: &p.guard,
                budget: &mut p.budget,
            });
            let mut terrain = Terrain {
                source: terrain,
                binding: &binding,
            };
            let mut collision = store::StaticStore::new(
                Collision {
                    source: collision,
                    binding: &binding,
                },
                &parents,
                store::Options {
                    load_path_only_models: true,
                    ..Default::default()
                },
                limits_source.store,
            )
            .map_err(super::Error::Store)?;
            pipeline::run(
                &p.plan,
                &mut pipeline::Sources {
                    terrain: &mut terrain,
                    collision: &mut collision,
                },
                types,
                connections,
                request,
                limits,
            )
            .map_err(super::Error::Pipeline)
        })();
        p.budget.release(&parents);
        if !result.as_ref().is_err_and(|e| e.changed()) {
            p.guard.verify_roots(&mut p.budget)?;
        }
        result
    })();
    if let Err(e) = result.as_ref()
        && e.changed()
    {
        p.failed = true;
    }
    result
}
