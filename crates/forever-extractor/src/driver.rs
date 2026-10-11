// SPDX-License-Identifier: GPL-3.0-or-later
// Authored 2026-10-11: serial Rust-local producers behind the common package.
use crate::{
    Error,
    plan::{Plan, Selection},
};
use forever_casc::{
    cameras,
    gametables::catalog as gt,
    integrity,
    local_store::{LocalStore, OpenOptions},
    package::{self, Component, Context, Run},
    root, terrain,
};
use forever_db2::{CharacterTable, DecodedTable};
use forever_navigation::{inventory, models, pipeline, recast, tilebuilder::Coordinate};
use std::path::Path;

const DB2_FILE_BYTES: u64 = 16 * 1024 * 1024;
const DB2_TOTAL_BYTES: u64 = 16 * 1024 * 1024 * 1024;

/// Admit the exact explicitly selected physical profile, offline. Opening alone
/// does not prove that the selected files are present or decodable.
pub fn open(plan: &Plan, client: &Path) -> Result<LocalStore, Error> {
    open_inner(plan, client, None)
}

/// Explicit v2 capability only. The original open entry point never enables
/// decryption implicitly; missing or unselected keys reject before client I/O.
pub fn open_with_keys(
    plan: &Plan,
    client: &Path,
    keys: forever_casc::keys::Keyring,
) -> Result<LocalStore, Error> {
    open_inner(plan, client, Some(keys))
}

fn open_inner(
    plan: &Plan,
    client: &Path,
    keys: Option<forever_casc::keys::Keyring>,
) -> Result<LocalStore, Error> {
    plan.validate()?;
    validate_keys(plan, keys.is_some())?;
    let options = OpenOptions {
        locale: plan.locale,
        index_layouts: [plan.index_layout; 16],
        root_layout: plan.root_layout,
        limits: Default::default(),
    };
    let mut store = match keys {
        Some(keys) => LocalStore::open_with_keys(client, options, keys),
        None => LocalStore::open(client, options),
    }
    .map_err(|e| Error::from(e.code()))?;
    if store.build().build != plan.build {
        return Err("product_build_mismatch".into());
    }
    if plan.contains(Component::Terrain) || plan.contains(Component::Collision) {
        store
            .enable_names(root::name::Limits::default())
            .map_err(|e| Error::from(e.code()))?;
    }
    Ok(store)
}

fn validate_keys(plan: &Plan, present: bool) -> Result<(), Error> {
    match (plan.requires_keys(), present) {
        (true, false) => Err("product_keys_required".into()),
        (false, true) => Err("product_keys_forbidden".into()),
        _ => Ok(()),
    }
}

pub fn context(plan: &Plan, store: &LocalStore, producer: [u8; 32]) -> Result<Context, Error> {
    plan.validate()?;
    if store.build().build != plan.build {
        return Err("product_build_mismatch".into());
    }
    let context = Context {
        build: plan.build,
        locale: plan.locale,
        components: plan.components,
        source_sha256: store.content_identity(),
        plan_sha256: integrity::digest(plan.render().as_bytes()),
        producer_sha256: producer,
    };
    context.validate()?;
    Ok(context)
}

/// Failed writes/producer errors retain the common marker and replay state.
/// Resume verifies the same logical collection, exact plan and producer binary;
/// unfinished components rerun completely and replay all previously written files.
pub fn extract(
    plan: &Plan,
    client: &Path,
    output: &Path,
    producer: [u8; 32],
    resume: bool,
) -> Result<package::Summary, Error> {
    extract_inner(plan, client, output, producer, resume, None)
}

/// The key input is explicitly loaded once and its source file is protected
/// from output writes. Key material and source paths never enter the package.
pub fn extract_with_keys(
    plan: &Plan,
    client: &Path,
    output: &Path,
    producer: [u8; 32],
    resume: bool,
    keys: crate::input::SuppliedKeys,
) -> Result<package::Summary, Error> {
    extract_inner(plan, client, output, producer, resume, Some(keys))
}

fn extract_inner(
    plan: &Plan,
    client: &Path,
    output: &Path,
    producer: [u8; 32],
    resume: bool,
    keys: Option<crate::input::SuppliedKeys>,
) -> Result<package::Summary, Error> {
    let (mut store, key_source) = match keys {
        Some(keys) => (
            open_with_keys(plan, client, keys.keyring)?,
            Some(keys.source),
        ),
        None => (open(plan, client)?, None),
    };
    let context = context(plan, &store, producer)?;
    let data = client.join("Data");
    let archives = data.join("data");
    let configs = data.join("config");
    let protected = [
        client,
        data.as_path(),
        archives.as_path(),
        configs.as_path(),
        key_source.as_deref().unwrap_or(client),
    ];
    let protected = &protected[..if key_source.is_some() { 5 } else { 4 }];
    let run = if resume {
        Run::resume(output, context, protected, Default::default())?
    } else {
        Run::create(output, context, protected, Default::default())?
    };
    execute(plan, &mut store, output, run)
}

#[cfg(test)]
mod keys_tests;

fn execute(
    plan: &Plan,
    store: &mut LocalStore,
    output: &Path,
    mut run: Run,
) -> Result<package::Summary, Error> {
    let completed = run.completed_components()?;
    for component in Component::ALL {
        if !plan.contains(component) || completed & component.bit() != 0 {
            continue;
        }
        let result = match component {
            Component::Db2 => db2(plan, store, &mut run),
            Component::GameTables => game_tables(store, &mut run),
            Component::Cameras => cameras(store, &mut run),
            Component::Terrain => maps(plan, store, &mut run),
            Component::Collision => vmaps(store, &mut run),
            Component::Navigation => mmaps(plan, store, output, &mut run),
        };
        result.map_err(|mut error| {
            error.component = Some(component);
            error
        })?;
        store
            .verify_unchanged()
            .map_err(|e| Error::from(e.code()))?;
        // Each producer completed, including the validated empty-world case.
        run.finish_component(component)?;
    }
    store
        .verify_unchanged()
        .map_err(|e| Error::from(e.code()))?;
    Ok(run.finish()?)
}

fn read(store: &mut LocalStore, id: u32, limit: u64) -> Result<Vec<u8>, Error> {
    let file = store
        .read_file_data_id_with_limit(id, limit)
        .map_err(|e| Error::from(e.code()))?;
    if file.bytes.is_empty() || file.bytes.len() as u64 > limit {
        return Err("product_file_limit".into());
    }
    Ok(file.bytes)
}

pub(crate) fn validate_db2(name: &str, bytes: &[u8]) -> Result<(), Error> {
    let schema = CharacterTable::from_catalog(name)
        .map_err(|e| Error::from(e.code()))?
        .ok_or_else(|| Error::from("product_db2_schema"))?;
    DecodedTable::read(bytes, schema).map_err(|e| Error::from(e.code()))?;
    Ok(())
}
fn db2(plan: &Plan, store: &mut LocalStore, run: &mut Run) -> Result<(), Error> {
    let mut total = 0u64;
    for entry in &plan.db2 {
        let bytes = read(store, entry.id, DB2_FILE_BYTES)?;
        total = total
            .checked_add(bytes.len() as u64)
            .filter(|n| *n <= DB2_TOTAL_BYTES)
            .ok_or_else(|| Error::from("product_db2_total_limit"))?;
        validate_db2(&entry.name, &bytes)?;
        run.put(
            Component::Db2,
            &format!("dbc/{}/{}.db2", plan.locale.name(), entry.name),
            &bytes,
        )?;
    }
    Ok(())
}
fn game_tables(store: &mut LocalStore, run: &mut Run) -> Result<(), Error> {
    for table in gt::Table::ALL {
        let bytes = read(
            store,
            table.file_id(),
            forever_casc::gametables::MAX_FILE_BYTES as u64,
        )?;
        if table.value_columns().is_some() {
            gt::parse_numeric(&bytes, table).map_err(|e| Error::from(e.code()))?;
        }
        run.put(Component::GameTables, table.relative_path(), &bytes)?;
    }
    Ok(())
}
fn cameras(store: &mut LocalStore, run: &mut Run) -> Result<(), Error> {
    let source = read(
        store,
        cameras::SOURCE_FILE_ID,
        cameras::MAX_SOURCE_BYTES as u64,
    )?;
    let plan = cameras::discover(&source)?;
    let mut total = source.len();
    for &id in plan.file_ids() {
        let bytes = read(store, id, cameras::MAX_FILE_BYTES as u64)?;
        total = total
            .checked_add(bytes.len())
            .filter(|n| *n <= cameras::MAX_TOTAL_BYTES)
            .ok_or_else(|| Error::from("camera_policy_limit"))?;
        run.put(Component::Cameras, &cameras::relative_path(id), &bytes)?;
    }
    Ok(())
}

struct TerrainSink<'a> {
    run: &'a mut Run,
    error: Option<Error>,
}
impl TerrainSink<'_> {
    fn put(&mut self, path: String, bytes: &[u8]) -> Result<(), terrain::run::SinkError> {
        self.run
            .put(Component::Terrain, &path, bytes)
            .map(|_| ())
            .map_err(|e| {
                self.error = Some(e.into());
                terrain::run::SinkError
            })
    }
}
impl terrain::run::Sink for TerrainSink<'_> {
    fn tile(
        &mut self,
        context: forever_terrain::conversion::Context,
        _: terrain::run::SourceFacts,
        tile: &forever_terrain::conversion::ConvertedTile,
    ) -> Result<(), terrain::run::SinkError> {
        let path = terrain::plan::map_relative_path(context.map_id, context.tile_x, context.tile_y)
            .map_err(|_| terrain::run::SinkError)?;
        self.put(path, tile.bytes())
    }
    fn tile_list(
        &mut self,
        map: u32,
        bytes: &[u8; terrain::tilelist::BYTES],
    ) -> Result<(), terrain::run::SinkError> {
        self.put(format!("maps/{map:04}.tilelist"), bytes)
    }
}
fn maps(plan: &Plan, store: &mut LocalStore, run: &mut Run) -> Result<(), Error> {
    let mut sink = TerrainSink { run, error: None };
    let ids;
    let tiles;
    let selection = match plan.terrain {
        Selection::All => terrain::plan::Selection::AllMaps,
        Selection::Map(id) => {
            ids = [id];
            terrain::plan::Selection::MapIds(&ids)
        }
        Selection::Tile { map, x, y } => {
            tiles = [terrain::plan::TileRequest { map_id: map, x, y }];
            terrain::plan::Selection::Tiles(&tiles)
        }
        Selection::None => return Err("product_terrain_selection".into()),
    };
    terrain::run::run(store, selection, plan.pack, Default::default(), &mut sink)
        .map_err(|failure| sink.error.unwrap_or_else(|| failure.error.code().into()))?;
    Ok(())
}
fn vmaps(store: &mut LocalStore, run: &mut Run) -> Result<(), Error> {
    use forever_casc::collision::{assemble, gameobjects};
    let startup = gameobjects::extract_startup_fdid(store, Default::default())
        .map_err(|e| Error::from(e.code()))?;
    let completed =
        assemble::extract(startup, Default::default()).map_err(|e| Error::from(e.code()))?;
    for artifact in completed.artifacts() {
        run.put(
            Component::Collision,
            &format!("vmaps/{}", artifact.name()),
            artifact.bytes(),
        )?;
    }
    Ok(())
}
struct NavigationSink<'a>(&'a mut Run);
impl inventory::fs::run::Sink for NavigationSink<'_> {
    type Error = Error;
    fn emit(&mut self, event: inventory::fs::run::Event<'_>) -> Result<(), Error> {
        use inventory::fs::run::Event;
        match event {
            Event::Global { map_id, bytes } => {
                self.0.put(
                    Component::Navigation,
                    &format!("mmaps/{map_id:04}.mmap"),
                    bytes,
                )?;
            }
            Event::Tile {
                map_id,
                tile,
                outcome,
            } => {
                if let pipeline::Outcome::Tile { bytes, .. } = outcome {
                    self.0.put(
                        Component::Navigation,
                        &format!(
                            "mmaps/{map_id:04}_{:02}_{:02}.mmtile",
                            tile.row, tile.column
                        ),
                        bytes,
                    )?;
                }
            }
        }
        Ok(())
    }
}
fn mmaps(plan: &Plan, store: &mut LocalStore, output: &Path, run: &mut Run) -> Result<(), Error> {
    use forever_db2::{catalog, terrain_sources};
    use inventory::fs::run::{self as nav, Cause};
    let map = read(
        store,
        catalog::find("Map").unwrap().file_data_id,
        DB2_FILE_BYTES,
    )?;
    let source = terrain_sources::read_navigation_maps(&map).map_err(|e| Error::from(e.code()))?;
    let liquid = read(
        store,
        catalog::find("LiquidType").unwrap().file_data_id,
        DB2_FILE_BYTES,
    )?;
    let liquid = terrain_sources::read_liquid_types(&liquid).map_err(|e| Error::from(e.code()))?;
    let types = models::liquid::Table::from_source(&liquid, Default::default())
        .map_err(|e| Error::from(e.code()))?;
    let selection = match plan.navigation {
        Selection::All => nav::Selection::All,
        Selection::Map(id) => nav::Selection::Map(id),
        Selection::Tile { map, x, y } => nav::Selection::Tile {
            map_id: map,
            tile: Coordinate { row: y, column: x },
        },
        Selection::None => return Err("product_navigation_selection".into()),
    };
    let map = match selection {
        nav::Selection::All => None,
        nav::Selection::Map(id) | nav::Selection::Tile { map_id: id, .. } => Some(id),
    };
    // A complete empty Map source has no discoverable world. Prove the empty
    // pure plan instead of inventing temporary maps/vmaps package artifacts.
    // Explicit Map/Tile requests remain errors, including in an empty world.
    if source.direct_records().is_empty() {
        let empty = inventory::Plan::from_source(
            &source,
            &[],
            plan.build,
            inventory::Options::default(),
            Default::default(),
        )
        .map_err(|e| Error::from(e.code()))?;
        if !matches!(selection, nav::Selection::All) || !empty.maps().is_empty() {
            return Err("product_navigation_selection".into());
        }
        store
            .verify_unchanged()
            .map_err(|e| Error::from(e.code()))?;
        return Ok(());
    }
    let mut input = inventory::fs::Package::open(
        &output.join("maps"),
        &output.join("vmaps"),
        &source,
        plan.build,
        inventory::Options {
            map,
            ..Default::default()
        },
        Default::default(),
    )
    .map_err(|e| Error::from(e.code()))?;
    nav::execute(
        &mut input,
        types.types(),
        &[],
        nav::Options {
            selection,
            big_base_unit: plan.big_base_unit,
            skip_liquids: plan.skip_liquids,
            slopes: recast::Slopes {
                maximum: plan.maximum_slope as f32,
                non_steep: plan.non_steep_slope as f32,
            },
        },
        Default::default(),
        &mut NavigationSink(run),
    )
    .map_err(|failure| match failure.cause {
        Cause::Run(error) => error.code().into(),
        Cause::Sink(error) => error,
    })?;
    Ok(())
}
