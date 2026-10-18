// SPDX-License-Identifier: GPL-3.0-or-later
// Authored 2026-10-11: serial Rust-local producers behind the common package.
use crate::{
    Error,
    plan::{CollisionStorage, Plan, Selection},
};
use forever_casc::{
    cameras,
    gametables::catalog as gt,
    integrity,
    local_store::{LocalStore, OpenOptions},
    package::{self, Component, Context, Run},
    root, terrain,
};
use forever_db2::{CharacterTable, DecodedTable, HEADER_SIZE, Header, RecordError};
use forever_navigation::{inventory, models, offmesh, pipeline, recast, tilebuilder::Coordinate};
#[cfg(target_os = "linux")]
use inventory::fs as navigation_input;
#[cfg(windows)]
use inventory::windows as navigation_input;
use std::path::Path;

#[cfg(target_os = "linux")]
mod disk;
mod execution;
#[cfg(target_os = "linux")]
pub(crate) mod terrain_v12;
#[cfg(target_os = "linux")]
pub(crate) mod terrain_v13;
#[cfg(windows)]
mod windows;
#[cfg(windows)]
use windows::extract_inner;

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
    admit(plan)?;
    validate_keys(plan, keys.is_some())?;
    if plan.is_v9() && forever_casc::admit(client).map_err(|e| Error::from(e.code()))? != plan.build
    {
        return Err("product_build_mismatch".into());
    }
    let options = OpenOptions {
        locale: plan.locale,
        index_layouts: [plan.index_layout; 16],
        root_layout: plan.root_layout,
        limits: Default::default(),
    };
    let mut store = if plan.is_v15() || plan.is_v16() {
        #[cfg(target_os = "linux")]
        {
            if forever_casc::admit_db2_70450(client).map_err(|e| Error::from(e.code()))? != 70450 {
                return Err("product_build_mismatch".into());
            }
            // validate_keys rejects any supplied key before this branch.
            LocalStore::open_db2_70450(client, options)
        }
        #[cfg(not(target_os = "linux"))]
        {
            return Err("product_platform".into());
        }
    } else if plan.is_v14() {
        #[cfg(target_os = "linux")]
        {
            if forever_casc::admit_db2_70425(client).map_err(|e| Error::from(e.code()))? != 70425 {
                return Err("product_build_mismatch".into());
            }
            // validate_keys rejects any supplied key before this branch.
            LocalStore::open_db2_70425(client, options)
        }
        #[cfg(not(target_os = "linux"))]
        {
            return Err("product_platform".into());
        }
    } else if plan.is_v10() || plan.is_v11() || plan.is_available_terrain() {
        #[cfg(target_os = "linux")]
        {
            if forever_casc::admit_db2_70374(client).map_err(|e| Error::from(e.code()))?
                != plan.build
            {
                return Err("product_build_mismatch".into());
            }
            match keys {
                Some(keys) => LocalStore::open_db2_70374_with_keys(client, options, keys),
                None => LocalStore::open_db2_70374(client, options),
            }
        }
        #[cfg(not(target_os = "linux"))]
        {
            return Err("product_platform".into());
        }
    } else {
        match keys {
            Some(keys) => LocalStore::open_with_keys(client, options, keys),
            None => LocalStore::open(client, options),
        }
    }
    .map_err(|e| Error::from(e.code()))?;
    if store.build().build != plan.build {
        return Err("product_build_mismatch".into());
    }
    if !plan.is_available_terrain()
        && (plan.contains(Component::Terrain) || plan.contains(Component::Collision))
    {
        store
            .enable_names(root::name::Limits::default())
            .map_err(|e| Error::from(e.code()))?;
    }
    Ok(store)
}

/// Admit the product execution platform before client, key or output I/O.
/// Plans remain portable to inspect without a client. Windows supports all
/// resident producers; the separately versioned Linux disk profile does not
/// acquire paths or silently select a resident fallback there.
pub fn admit(plan: &Plan) -> Result<(), Error> {
    plan.validate()?;
    #[cfg(not(target_os = "linux"))]
    if plan.map_domain.is_some()
        || plan.is_gt_cameras()
        || plan.is_v10()
        || plan.is_available_terrain()
        || plan.is_v14()
        || plan.is_v15()
        || plan.is_v16()
    {
        return Err("product_platform".into());
    }
    #[cfg(windows)]
    if !matches!(plan.collision_storage, CollisionStorage::Resident) {
        return Err("product_disk_unsupported".into());
    }
    Ok(())
}

/// Inspect a client while retaining the supplied key's read lease through
/// source/context admission. No key values or physical key identity enter it.
pub fn inspect(
    plan: &Plan,
    client: &Path,
    producer: [u8; 32],
    keys: Option<crate::input::SuppliedKeys>,
) -> Result<Context, Error> {
    admit(plan)?;
    validate_keys(plan, keys.is_some())?;
    #[cfg(windows)]
    {
        windows::inspect(plan, client, producer, keys)
    }
    #[cfg(target_os = "linux")]
    {
        let store = match keys {
            Some(keys) => open_with_keys(plan, client, keys.into_keyring()),
            None => open(plan, client),
        }?;
        context(plan, &store, producer)
    }
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
    extract_inner(plan, client, output, producer, resume, None, None)
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
    extract_inner(plan, client, output, producer, resume, Some(keys), None)
}

/// Explicit Linux leased disk-world collision extraction. Scratch is a private
/// operator-owned directory; each attempt retains a new exclusive namespace.
/// Plan budgets, source/output boundaries and physical space admit before client
/// acquisition or common package creation. Resume reruns unfinished producers.
pub fn extract_disk(
    plan: &Plan,
    client: &Path,
    output: &Path,
    producer: [u8; 32],
    resume: bool,
    keys: Option<crate::input::SuppliedKeys>,
    scratch: &Path,
) -> Result<package::Summary, Error> {
    extract_inner(plan, client, output, producer, resume, keys, Some(scratch))
}

#[cfg(target_os = "linux")]
fn extract_inner(
    plan: &Plan,
    client: &Path,
    output: &Path,
    producer: [u8; 32],
    resume: bool,
    keys: Option<crate::input::SuppliedKeys>,
    scratch: Option<&Path>,
) -> Result<package::Summary, Error> {
    plan.validate()?;
    let preflight = match (plan.collision_storage, scratch) {
        (CollisionStorage::Resident, None) => None,
        (CollisionStorage::Resident, Some(_)) => return Err("product_scratch_forbidden".into()),
        (CollisionStorage::LeasedDiskV1(_), None) => return Err("product_scratch_required".into()),
        (CollisionStorage::LeasedDiskV1(budget), Some(scratch)) => {
            validate_keys(plan, keys.is_some())?;
            Some(disk::Preflight::open(
                budget,
                scratch,
                output,
                client,
                keys.as_ref().map(|keys| keys.source.as_path()),
            )?)
        }
    };
    let (mut store, key_source) = match keys {
        Some(keys) => (
            open_with_keys(plan, client, keys.keyring)?,
            Some(keys.source),
        ),
        None => (open(plan, client)?, None),
    };
    // Validate complete Map and all selected projections before creating or
    // resuming package state. Retained compact facts remain charged by each
    // producer; source buffers and graph scratch are dropped here.
    let domain = prepare_domain(plan, &mut store)?;
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
    let mut package_limits = package::Limits::default();
    if plan.is_available_terrain() {
        package_limits = terrain_v12::package_limits();
    }
    if let CollisionStorage::LeasedDiskV1(budget) = plan.collision_storage {
        // Physical output admission covers all selected producers, not only
        // vmaps. The canonical v3 output budget also bounds common replay.
        package_limits.total_bytes = budget.output_bytes;
    }
    let run = if resume {
        Run::resume(output, context, protected, package_limits)?
    } else {
        Run::create(output, context, protected, package_limits)?
    };
    execution::execute(
        plan,
        &mut store,
        output,
        run,
        LinuxExecution {
            preflight,
            domain,
            context,
        },
    )
}

#[cfg(target_os = "linux")]
fn prepare_domain(
    plan: &Plan,
    store: &mut LocalStore,
) -> Result<Option<crate::map_domain::Domain>, Error> {
    // v14 derives its declared domain once from the genuine available Map owner.
    // Do not reopen a complete Map or manufacture strict SourceFacts here.
    if plan.is_v14() || plan.is_v15() || plan.is_v16() {
        return Ok(None);
    }
    let Some(requested) = plan.map_domain else {
        return Ok(None);
    };
    let bytes = read(store, terrain::run::MAP_ID, DB2_FILE_BYTES)?;
    let peak = crate::map_domain::PREPARATION_BYTES
        .checked_add(bytes.capacity().saturating_sub(bytes.len()))
        .and_then(|n| n.checked_add(store.stats().resident_bytes))
        .ok_or("product_map_domain_limit")?;
    if peak > 2 * 1024 * 1024 * 1024 {
        return Err("product_map_domain_limit".into());
    }
    let domain =
        crate::map_domain::Domain::derive(&bytes, requested, plan.contains(Component::Navigation))?;
    store
        .verify_unchanged()
        .map_err(|e| Error::from(e.code()))?;
    Ok(Some(domain))
}

#[cfg(test)]
mod keys_tests;

/// Linux owns the admitted scratch lease. Its guards retain the existing
/// post-producer/final positions; resident execution has no extra input lease.
#[cfg(target_os = "linux")]
struct LinuxExecution {
    context: Context,
    preflight: Option<disk::Preflight>,
    domain: Option<crate::map_domain::Domain>,
}

#[cfg(target_os = "linux")]
impl execution::Adapter for LinuxExecution {
    fn extract_terrain(
        &self,
        plan: &Plan,
        store: &mut LocalStore,
        run: &mut Run,
    ) -> Result<(), Error> {
        if plan.is_v13() {
            terrain_v13::extract(plan, store, run, self.context)
        } else if plan.is_v12() {
            terrain_v12::extract(plan, store, run, self.context)
        } else {
            maps(plan, store, run, self.domain.as_ref())
        }
    }
    fn map_domain(&self) -> Option<&crate::map_domain::Domain> {
        self.domain.as_ref()
    }
    fn verify_inputs(&self) -> Result<(), Error> {
        if let Some(disk) = &self.preflight {
            disk.verify_unchanged()?;
        }
        Ok(())
    }

    fn extract_collision(
        &self,
        plan: &Plan,
        store: &mut LocalStore,
        run: &mut Run,
    ) -> Result<(), Error> {
        if plan.is_v14() {
            return self
                .preflight
                .as_ref()
                .ok_or("product_scratch_required")?
                .run_compatible_70425(store, run, plan.map_domain, self.context);
        }
        if plan.is_v15() {
            return self
                .preflight
                .as_ref()
                .ok_or("product_scratch_required")?
                .run_compatible_70450(store, run, plan.map_domain, self.context);
        }
        match &self.preflight {
            Some(disk) => match &self.domain {
                Some(domain) => disk.run_selected(store, run, domain),
                None => disk.run(store, run),
            },
            None => vmaps(store, run),
        }
    }
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

pub(crate) fn validate_db2(plan: &Plan, name: &str, bytes: &[u8]) -> Result<(), Error> {
    let schema = match plan.schema_catalog {
        crate::plan::SchemaCatalog::PinnedMetadataV1 => CharacterTable::from_catalog(name),
        catalog => CharacterTable::from_catalog_id(catalog.id(), name),
    }
    .map_err(|e| Error::from(e.code()))?
    .ok_or_else(|| Error::from("product_db2_schema"))?;

    // Preserve v1-v3's regular-table verifier exactly. The build-specific v4/v5
    // profiles additionally admit bounded catalog-aware readers and a raw-byte
    // validation fallback for non-UTF-8 regular strings. Product DB2 artifacts
    // preserve the original source bytes; this fallback returns no text values.
    let target_catalog = matches!(
        plan.schema_catalog,
        crate::plan::SchemaCatalog::WowDbDefs70205V1
            | crate::plan::SchemaCatalog::WowDbDefs70338V1
            | crate::plan::SchemaCatalog::WowDbDefs70374V1
            | crate::plan::SchemaCatalog::WowDbDefs70425V1
            | crate::plan::SchemaCatalog::WowDbDefs70450V1
    );
    let validation = (|| {
        if target_catalog && bytes.len() >= HEADER_SIZE {
            let header_bytes: &[u8; HEADER_SIZE] = bytes[..HEADER_SIZE]
                .try_into()
                .map_err(|_| RecordError::Metadata)?;
            let header = Header::parse(header_bytes).map_err(|_| RecordError::Metadata)?;
            if header.section_count > 1 {
                return DecodedTable::read_regular_sections_from_catalog(
                    bytes,
                    plan.schema_catalog.id(),
                    name,
                    16,
                )
                .map(|_| ());
            }
            if DecodedTable::read_target_high_volume_from_catalog(
                bytes,
                plan.schema_catalog.id(),
                name,
            )?
            .is_some()
            {
                return Ok(());
            }
        }
        DecodedTable::read(bytes, schema).map(|_| ())
    })();

    match validation {
        Err(RecordError::String) if target_catalog => {
            DecodedTable::validate_catalog_raw_source_bytes(bytes, plan.schema_catalog.id(), name)
                .map_err(|error| Error::from(error.code()))
        }
        Err(RecordError::Unsupported)
            if target_catalog
                && matches!(
                    name,
                    "CollectableSourceEncounterSparse"
                        | "CollectableSourceQuestSparse"
                        | "CollectableSourceVendorSparse"
                ) =>
        {
            DecodedTable::validate_catalog_raw_source_bytes(bytes, plan.schema_catalog.id(), name)
                .map_err(|error| Error::from(error.code()))
        }
        result => result.map_err(|error| Error::from(error.code())),
    }
}
fn db2(
    plan: &Plan,
    store: &mut LocalStore,
    run: &mut Run,
    domain: Option<&crate::map_domain::Domain>,
) -> Result<(), Error> {
    let mut total = 0u64;
    for entry in &plan.db2 {
        let cap = if plan.is_gt_cameras()
            || plan.is_v10()
            || plan.is_v16()
            || plan.is_available_terrain()
        {
            db2_remaining(total)?
        } else {
            DB2_FILE_BYTES
        };
        let bytes = read(store, entry.id, cap)?;
        total = total
            .checked_add(bytes.len() as u64)
            .filter(|n| *n <= DB2_TOTAL_BYTES)
            .ok_or_else(|| Error::from("product_db2_total_limit"))?;
        validate_db2(plan, &entry.name, &bytes)?;
        if entry.name == "Map"
            && let Some(domain) = domain
        {
            domain.verify_source(&bytes)?;
        }
        run.put(
            Component::Db2,
            &format!("dbc/{}/{}.db2", plan.locale.name(), entry.name),
            &bytes,
        )?;
    }
    Ok(())
}
fn game_tables(plan: &Plan, store: &mut LocalStore, run: &mut Run) -> Result<(), Error> {
    if plan.is_v9() {
        forever_casc::gametables::contract::CatalogContract::Build70338V1
            .admit_build(plan.build)?;
    }
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
fn cameras(product: &Plan, store: &mut LocalStore, run: &mut Run) -> Result<(), Error> {
    let source = read(
        store,
        cameras::SOURCE_FILE_ID,
        cameras::MAX_SOURCE_BYTES as u64,
    )?;
    let plan = camera_plan(product, run, &source)?;
    let mut total = source.len();
    for &id in plan.file_ids() {
        let limit = if product.is_gt_cameras() {
            camera_remaining(total)?
        } else {
            cameras::MAX_FILE_BYTES as u64
        };
        let bytes = read(store, id, limit)?;
        total = total
            .checked_add(bytes.len())
            .filter(|n| *n <= cameras::MAX_TOTAL_BYTES)
            .ok_or_else(|| Error::from("camera_policy_limit"))?;
        run.put(Component::Cameras, &cameras::relative_path(id), &bytes)?;
    }
    Ok(())
}

fn camera_plan(
    product: &Plan,
    _run: &mut Run,
    source: &[u8],
) -> Result<cameras::CameraPlan, Error> {
    #[cfg(target_os = "linux")]
    if product.is_gt_cameras() {
        _run.verify_sealed_artifact(&package::Artifact {
            component: Component::Db2,
            path: cameras::source_path(product.locale),
            bytes: source.len() as u64,
            sha256: integrity::digest(source),
        })?;
    }
    let discovered = if product.is_v11() {
        cameras::discover_70374(source)?
    } else if product.is_v9() {
        cameras::discover_with_contract(cameras::CameraContract::Build70338V1, source)?
    } else {
        cameras::discover(source)?
    };
    Ok(discovered)
}

fn db2_remaining(total: u64) -> Result<u64, Error> {
    DB2_TOTAL_BYTES
        .checked_sub(total)
        .map(|n| n.min(DB2_FILE_BYTES))
        .filter(|n| *n != 0)
        .ok_or_else(|| Error::from("product_db2_total_limit"))
}

fn camera_remaining(total: usize) -> Result<u64, Error> {
    cameras::MAX_TOTAL_BYTES
        .checked_sub(total)
        .map(|remaining| remaining.min(cameras::MAX_FILE_BYTES))
        .filter(|remaining| *remaining != 0)
        .map(|remaining| remaining as u64)
        .ok_or_else(|| Error::from("camera_policy_limit"))
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
fn maps(
    plan: &Plan,
    store: &mut LocalStore,
    run: &mut Run,
    domain: Option<&crate::map_domain::Domain>,
) -> Result<(), Error> {
    let mut sink = TerrainSink { run, error: None };
    let ids;
    let tiles;
    let selection = if let Some(domain) = domain {
        terrain::plan::Selection::MapIds(domain.ids())
    } else {
        match plan.terrain {
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
        }
    };
    let mut limits = terrain::run::Limits::default();
    if let Some(domain) = domain {
        limits.max_memory_bytes = limits
            .max_memory_bytes
            .checked_sub(domain.resident_bytes())
            .ok_or("product_map_domain_limit")?;
    }
    terrain::run::run(store, selection, plan.pack, limits, &mut sink)
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
impl navigation_input::run::Sink for NavigationSink<'_> {
    type Error = Error;
    fn emit(&mut self, event: navigation_input::run::Event<'_>) -> Result<(), Error> {
        use navigation_input::run::Event;
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
fn mmaps(
    plan: &Plan,
    store: &mut LocalStore,
    output: &Path,
    run: &mut Run,
    domain: Option<&crate::map_domain::Domain>,
) -> Result<(), Error> {
    use forever_db2::{catalog, terrain_sources};
    use navigation_input::run::{self as nav, Cause};
    let map = read(
        store,
        catalog::find("Map").unwrap().file_data_id,
        DB2_FILE_BYTES,
    )?;
    if let Some(domain) = domain {
        domain.verify_source(&map)?;
    }
    let source = terrain_sources::read_navigation_maps(&map).map_err(|e| Error::from(e.code()))?;
    let connections = navigation_connections(plan, source.direct_records().is_empty())?;
    if domain.is_some() {
        drop(map);
    }
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
    // Navigation dependencies use their own original candidate sets. A child
    // with only inherited indexes emits no invented global/tile; its ancestors
    // are generated in this same domain, with the same collision ID context.
    let (source, selected_rows) = match domain {
        Some(domain) => {
            let rows = domain.navigation_rows(&source)?;
            drop(source);
            (None, Some(rows))
        }
        None => (Some(source), None),
    };
    let caller_bytes = match (domain, selected_rows.as_ref()) {
        (Some(domain), Some(rows)) => domain.navigation_resident_bytes(rows.capacity())?,
        _ => 0,
    };
    let mut discovery_limits = navigation_input::Limits::default();
    let mut run_limits = nav::Limits::default();
    if domain.is_some() {
        discovery_limits.heap_bytes = discovery_limits
            .heap_bytes
            .checked_sub(caller_bytes)
            .ok_or("product_map_domain_limit")?;
        run_limits.pipeline.heap_bytes = run_limits
            .pipeline
            .heap_bytes
            .checked_sub(caller_bytes)
            .ok_or("product_map_domain_limit")?;
    }
    let mut input = if let Some(rows) = selected_rows.as_ref() {
        navigation_input::Package::open_rows(
            &output.join("maps"),
            &output.join("vmaps"),
            rows,
            plan.build,
            inventory::Options::default(),
            discovery_limits,
        )
    } else {
        navigation_input::Package::open(
            &output.join("maps"),
            &output.join("vmaps"),
            source.as_ref().ok_or("product_map_domain_projection")?,
            plan.build,
            inventory::Options {
                map,
                ..Default::default()
            },
            discovery_limits,
        )
    }
    .map_err(|e| Error::from(e.code()))?;
    if let Some(domain) = domain
        && (input
            .plan()
            .maps()
            .iter()
            .any(|entry| !domain.contains(entry.map_id()))
            || input.plan().maps().len() != domain.ids().len())
    {
        return Err("product_map_domain_projection".into());
    }
    let selection = if domain.is_some() {
        nav::Selection::All
    } else {
        selection
    };
    nav::execute(
        &mut input,
        types.types(),
        &connections,
        nav::Options {
            selection,
            big_base_unit: plan.big_base_unit,
            skip_liquids: plan.skip_liquids,
            slopes: recast::Slopes {
                maximum: plan.maximum_slope as f32,
                non_steep: plan.non_steep_slope as f32,
            },
        },
        run_limits,
        &mut NavigationSink(run),
    )
    .map_err(|failure| match failure.cause {
        Cause::Run(error) => error.code().into(),
        Cause::Sink(error) => error,
    })?;
    Ok(())
}

fn navigation_connections(plan: &Plan, map_empty: bool) -> Result<Vec<offmesh::Source>, Error> {
    let connections = plan.offmesh.as_deref().unwrap_or_default();
    if map_empty && !connections.is_empty() {
        return Err("product_offmesh_empty_map".into());
    }
    let mut sources = Vec::new();
    sources
        .try_reserve_exact(connections.len())
        .map_err(|_| Error::from("product_allocation"))?;
    sources.extend(connections.iter().map(|connection| connection.source()));
    Ok(sources)
}

#[cfg(test)]
mod offmesh_tests;

#[cfg(test)]
mod platform_tests;

#[cfg(all(test, target_os = "linux"))]
mod disk_admission_tests {
    use super::*;
    use crate::plan::DiskBudget;

    fn plan() -> Plan {
        let mut plan = Plan::parse("forever-product\t1\nbuild\t70205\nlocale\tenUS\nindex\tcontiguous\nroot\theader-50893-v2\ncomponents\tvmaps\nterrain\tnone\npack\ttrue\nnavigation\tnone\nbig-base-unit\tfalse\nskip-liquids\tfalse\nmaximum-slope\t55\nnon-steep-slope\t55\n").unwrap();
        plan.collision_storage = CollisionStorage::LeasedDiskV1(DiskBudget {
            raw_bytes: 1024,
            wdt_bytes: 1024,
            cache_bytes: 1024,
            placement_bytes: 1024,
            vmo_bytes: 1024,
            map_bytes: 1024,
            output_bytes: 1024,
        });
        plan
    }
    #[test]
    fn original_extract_entries_reject_disk_without_scratch_before_client_io() {
        let client = Path::new("/nonexistent-authored-disk-product-client");
        let output = Path::new("/nonexistent-authored-disk-product-output");
        assert_eq!(
            extract(&plan(), client, output, [0; 32], false)
                .unwrap_err()
                .code,
            "product_scratch_required"
        );
        let keys = crate::input::SuppliedKeys {
            keyring: forever_casc::keys::Keyring::parse(
                b"forever-tact-keys\t1\n0000000000000000\t00000000000000000000000000000000\n",
            )
            .unwrap(),
            source: Path::new("/nonexistent-authored-disk-product-keys").to_owned(),
        };
        assert_eq!(
            extract_with_keys(&plan(), client, output, [0; 32], false, keys)
                .unwrap_err()
                .code,
            "product_scratch_required"
        );
    }
    #[test]
    fn explicit_disk_entry_rejects_resident_before_client_io() {
        let mut plan = plan();
        plan.collision_storage = CollisionStorage::Resident;
        assert_eq!(
            extract_disk(
                &plan,
                Path::new("/nonexistent-authored-client"),
                Path::new("/nonexistent-authored-output"),
                [0; 32],
                false,
                None,
                Path::new("/nonexistent-authored-scratch")
            )
            .unwrap_err()
            .code,
            "product_scratch_forbidden"
        );
    }
    #[test]
    fn malformed_manual_budget_rejects_before_any_paths_are_opened() {
        let mut plan = plan();
        let CollisionStorage::LeasedDiskV1(mut budget) = plan.collision_storage else {
            unreachable!()
        };
        budget.output_bytes = 0;
        plan.collision_storage = CollisionStorage::LeasedDiskV1(budget);
        assert_eq!(
            extract_disk(
                &plan,
                Path::new("/nonexistent-authored-client"),
                Path::new("/nonexistent-authored-output"),
                [0; 32],
                false,
                None,
                Path::new("/nonexistent-authored-scratch")
            )
            .unwrap_err()
            .code,
            "product_plan_disk_budget"
        );
    }
}
#[cfg(test)]
mod v9_tests;

#[cfg(test)]
mod v11_tests;
