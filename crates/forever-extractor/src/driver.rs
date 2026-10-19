// SPDX-License-Identifier: GPL-3.0-or-later
// Modified by the RustyCore Extractors project, 2026-10-19.
// Authored 2026-10-11: serial Rust-local producers behind the common package.
use crate::{
    Error,
    plan::{CollisionStorage, Plan, Selection},
};
#[cfg(target_os = "linux")]
use forever_casc::local_store::{ProductDiagnosticPhase, ProductDiagnosticRequestKind};
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

#[cfg(all(test, target_os = "linux"))]
// CASC intentionally exports neither helper; the exact original source is
// included here to author a local synthetic archive, and is also used by the
// collision fixture module (cloning these cryptographic/checksum algorithms
// into another test implementation would be less auditable).
#[allow(clippy::duplicate_mod)]
#[path = "../../forever-casc/src/local_index/jenkins.rs"]
mod v21_test_jenkins;
#[cfg(all(test, target_os = "linux"))]
// See the matching Jenkins fixture module above; both allowances cover only
// duplicate test-only source inclusion, not production code or test assertions.
#[allow(clippy::duplicate_mod)]
#[path = "../../forever-casc/src/blte/md5.rs"]
mod v21_test_md5;

#[cfg(target_os = "linux")]
pub(crate) mod census_db2;
#[cfg(target_os = "linux")]
pub(crate) mod collision_geometry_v22;
#[cfg(target_os = "linux")]
mod disk;
mod execution;
#[cfg(target_os = "linux")]
pub(crate) mod inspect_db2;
#[cfg(target_os = "linux")]
pub(crate) mod probe_encoding;
#[cfg(target_os = "linux")]
pub(crate) mod terrain_v12;
#[cfg(target_os = "linux")]
pub(crate) mod terrain_v13;
#[cfg(target_os = "linux")]
pub(crate) mod terrain_v20;
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

/// Run the Linux-only redacted encoding probe. No package or component output
/// is created; the only write is the caller-selected trace file.
#[cfg(target_os = "linux")]
pub fn probe_encoding_command(
    plan: &Plan,
    client: &Path,
    output: &Path,
    file_data_ids: &[u32],
) -> Result<(usize, [u8; 32]), Error> {
    let summary = probe_encoding::run(plan, client, output, file_data_ids)?;
    Ok((summary.file_data_ids, summary.trace_sha256))
}

/// Write the Linux-only all-declaration keyless DB2 availability census.
#[cfg(target_os = "linux")]
pub fn census_db2_command(plan: &Plan, client: &Path, output: &Path) -> Result<usize, Error> {
    let summary = census_db2::run(plan, client, output)?;
    Ok(summary.declaration_count)
}

/// Write a deterministic, redacted structural/classification report for a
/// bounded set of 70450 DB2 declarations. No DB2 bytes are published.
#[cfg(target_os = "linux")]
pub fn inspect_db2_command(
    plan: &Plan,
    client: &Path,
    output: &Path,
    file_data_ids: &[u32],
) -> Result<usize, Error> {
    let summary = inspect_db2::run(plan, client, output, file_data_ids)?;
    Ok(summary.declaration_count)
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
    let mut store = if plan.is_v15()
        || plan.is_v16()
        || plan.is_v17()
        || plan.is_v18()
        || plan.is_v19()
        || plan.is_v20()
        || plan.is_v21()
        || plan.is_v22()
    {
        #[cfg(target_os = "linux")]
        {
            if forever_casc::admit_db2_70450(client).map_err(|e| Error::from(e.code()))? != 70450 {
                return Err("product_build_mismatch".into());
            }
            if plan.is_v17() || plan.is_v18() {
                let Some(keys) = keys else {
                    return Err("product_keys_required".into());
                };
                LocalStore::open_db2_70450_with_keys(client, options, keys)
            } else {
                // Disabled-key revisions reject supplied keys before this opener.
                LocalStore::open_db2_70450(client, options)
            }
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
        && !plan.is_v20()
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
        || plan.is_v17()
        || plan.is_v18()
        || plan.is_v20()
        || plan.is_v21()
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
    let diagnostic_output = if product_diagnostic_supported(plan) {
        std::env::var_os("FOREVER_PRODUCT_DIAGNOSTIC")
    } else {
        None
    };
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
    let product_diagnostic = if product_diagnostic_supported(plan) {
        let package_output = (plan.is_v18() || plan.is_v19() || plan.is_v21()).then_some(output);
        disk::product_diagnostic_for_db2(client, diagnostic_output, package_output)?
    } else {
        None
    };
    let diagnostic_enabled = product_diagnostic.is_some();
    let (mut store, key_source) = match keys {
        Some(keys) => (
            open_with_keys(plan, client, keys.keyring)?,
            Some(keys.source),
        ),
        None => (open(plan, client)?, None),
    };
    if let Some(observer) = product_diagnostic {
        store.set_product_diagnostic_observer(Some(observer));
        store.set_product_diagnostic_phase(ProductDiagnosticPhase::Db2);
    }
    // Validate complete Map and all selected projections before creating or
    // resuming package state. Retained compact facts remain charged by each
    // producer; source buffers and graph scratch are dropped here.
    let domain = match prepare_domain(plan, &mut store) {
        Ok(domain) => domain,
        Err(error) => {
            if diagnostic_enabled {
                store.finish_product_diagnostic(Some(error.code), false);
            }
            return Err(error);
        }
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
    let mut package_limits = package::Limits::default();
    if plan.is_v20() {
        package_limits = terrain_v20::package_limits();
    } else if plan.is_available_terrain() {
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
    let result = execution::execute(
        plan,
        &mut store,
        output,
        run,
        LinuxExecution {
            preflight,
            domain,
            context,
        },
    );
    if diagnostic_enabled {
        store.finish_product_diagnostic(result.as_ref().err().map(|error| error.code), false);
    }
    result
}

#[cfg(target_os = "linux")]
fn product_diagnostic_supported(plan: &Plan) -> bool {
    plan.is_v16() || plan.is_v17() || plan.is_v18() || plan.is_v19() || plan.is_v21()
}

#[cfg(target_os = "linux")]
fn prepare_domain(
    plan: &Plan,
    store: &mut LocalStore,
) -> Result<Option<crate::map_domain::Domain>, Error> {
    // v14 derives its declared domain once from the genuine available Map owner.
    // Do not reopen a complete Map or manufacture strict SourceFacts here.
    if plan.is_v14()
        || plan.is_v15()
        || plan.is_v16()
        || plan.is_v17()
        || plan.is_v18()
        || plan.is_v22()
    {
        return Ok(None);
    }
    let Some(requested) = plan.map_domain else {
        return Ok(None);
    };
    let bytes = if plan.is_v21() {
        let map_file_data_id = plan
            .db2
            .iter()
            .find(|entry| entry.name == "Map")
            .map(|entry| entry.id)
            .ok_or("product_plan_dependencies")?;
        let diagnostic_request = store.begin_product_diagnostic_request(
            ProductDiagnosticRequestKind::Db2Table,
            Some(map_file_data_id),
        );
        let file = match store.read_file_data_id_with_limit(map_file_data_id, DB2_FILE_BYTES) {
            Ok(file) => file,
            Err(error) => {
                store.record_product_diagnostic_failure(
                    diagnostic_request,
                    error,
                    matches!(error, forever_casc::local_store::Error::InputChanged),
                );
                return Err(Error::from(error.code()));
            }
        };
        if file.bytes.is_empty() || file.bytes.len() as u64 > DB2_FILE_BYTES {
            return Err("product_file_limit".into());
        }
        file.bytes
    } else {
        read(store, terrain::run::MAP_ID, DB2_FILE_BYTES)?
    };
    if plan.is_v21() {
        // Validate complete build-specific DB2 decoding before the common
        // package is created. The domain projections below also use strict
        // 70450 readers; partial availability APIs are never admitted here.
        validate_db2(plan, "Map", &bytes)?;
    }
    let peak = crate::map_domain::PREPARATION_BYTES
        .checked_add(bytes.capacity().saturating_sub(bytes.len()))
        .and_then(|n| n.checked_add(store.stats().resident_bytes))
        .ok_or("product_map_domain_limit")?;
    if peak > 2 * 1024 * 1024 * 1024 {
        return Err("product_map_domain_limit".into());
    }
    let domain = if plan.is_v21() {
        crate::map_domain::Domain::derive_70450(
            &bytes,
            requested,
            plan.contains(Component::Navigation),
        )?
    } else {
        crate::map_domain::Domain::derive(&bytes, requested, plan.contains(Component::Navigation))?
    };
    if plan.is_v21() {
        let liquid_file_data_id = plan
            .db2
            .iter()
            .find(|entry| entry.name == "LiquidType")
            .map(|entry| entry.id)
            .ok_or("product_plan_dependencies")?;
        let liquid = read(store, liquid_file_data_id, DB2_FILE_BYTES)?;
        validate_db2(plan, "LiquidType", &liquid)?;
        let records = forever_db2::terrain_sources::read_liquid_types_70450(&liquid)
            .map_err(|error| Error::from(error.code()))?;
        let _ = models::liquid::Table::from_source(&records, Default::default())
            .map_err(|error| Error::from(error.code()))?;
    }
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
        if plan.is_v20() {
            terrain_v20::extract(plan, store, run, self.context)
        } else if plan.is_v13() {
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
        if plan.is_v22() {
            let CollisionStorage::LeasedDiskV1(budget) = plan.collision_storage else {
                return Err("product_scratch_required".into());
            };
            return collision_geometry_v22::extract(
                plan,
                store,
                run,
                self.preflight.as_ref().ok_or("product_scratch_required")?,
                self.context,
                budget,
            );
        }
        if plan.is_v14() {
            return self
                .preflight
                .as_ref()
                .ok_or("product_scratch_required")?
                .run_compatible_70425(store, run, plan.map_domain, self.context);
        }
        if plan.is_v15() || plan.is_v18() {
            return self
                .preflight
                .as_ref()
                .ok_or("product_scratch_required")?
                .run_compatible_70450(store, run, plan.map_domain, self.context, plan.is_v18());
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
            let validation = if plan.schema_catalog == crate::plan::SchemaCatalog::WowDbDefs70450V1
            {
                DecodedTable::validate_catalog_raw_source_bytes_70450(bytes, name)
            } else {
                DecodedTable::validate_catalog_raw_source_bytes(
                    bytes,
                    plan.schema_catalog.id(),
                    name,
                )
            };
            validation.map_err(|error| Error::from(error.code()))
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
        let cap = db2_read_limit(plan, total)?;
        #[cfg(target_os = "linux")]
        let diagnostic_request = if product_diagnostic_supported(plan) {
            store.begin_product_diagnostic_request(
                ProductDiagnosticRequestKind::Db2Table,
                Some(entry.id),
            )
        } else {
            None
        };
        let file = match store.read_file_data_id_with_limit(entry.id, cap) {
            Ok(file) => file,
            Err(error) => {
                #[cfg(target_os = "linux")]
                if product_diagnostic_supported(plan) {
                    store.record_product_diagnostic_failure(
                        diagnostic_request,
                        error,
                        matches!(error, forever_casc::local_store::Error::InputChanged),
                    );
                }
                return Err(Error::from(error.code()));
            }
        };
        if file.bytes.is_empty() || file.bytes.len() as u64 > cap {
            return Err("product_file_limit".into());
        }
        let bytes = file.bytes;
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
        let limit = camera_read_limit(product, total)?;
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

fn db2_read_limit(plan: &Plan, total: u64) -> Result<u64, Error> {
    if plan.is_gt_cameras()
        || plan.is_v10()
        || plan.is_v16()
        || plan.is_v17()
        || plan.is_available_terrain()
    {
        db2_remaining(total)
    } else {
        Ok(DB2_FILE_BYTES)
    }
}

fn camera_remaining(total: usize) -> Result<u64, Error> {
    cameras::MAX_TOTAL_BYTES
        .checked_sub(total)
        .map(|remaining| remaining.min(cameras::MAX_FILE_BYTES))
        .filter(|remaining| *remaining != 0)
        .map(|remaining| remaining as u64)
        .ok_or_else(|| Error::from("camera_policy_limit"))
}

fn camera_read_limit(plan: &Plan, total: usize) -> Result<u64, Error> {
    if plan.is_gt_cameras() {
        camera_remaining(total)
    } else {
        Ok(cameras::MAX_FILE_BYTES as u64)
    }
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
    let map_file_data_id = if plan.is_v21() {
        plan.db2
            .iter()
            .find(|entry| entry.name == "Map")
            .map(|entry| entry.id)
            .ok_or("product_plan_dependencies")?
    } else {
        catalog::find("Map").unwrap().file_data_id
    };
    let map = read(store, map_file_data_id, DB2_FILE_BYTES)?;
    if let Some(domain) = domain {
        domain.verify_source(&map)?;
    }
    let source = if plan.is_v21() {
        terrain_sources::read_navigation_maps_70450(&map)
    } else {
        terrain_sources::read_navigation_maps(&map)
    }
    .map_err(|e| Error::from(e.code()))?;
    let connections = navigation_connections(plan, source.direct_records().is_empty())?;
    if domain.is_some() {
        drop(map);
    }
    let liquid_file_data_id = if plan.is_v21() {
        plan.db2
            .iter()
            .find(|entry| entry.name == "LiquidType")
            .map(|entry| entry.id)
            .ok_or("product_plan_dependencies")?
    } else {
        catalog::find("LiquidType").unwrap().file_data_id
    };
    let liquid = read(store, liquid_file_data_id, DB2_FILE_BYTES)?;
    let liquid = if plan.is_v21() {
        terrain_sources::read_liquid_types_70450(&liquid)
    } else {
        terrain_sources::read_liquid_types(&liquid)
    }
    .map_err(|e| Error::from(e.code()))?;
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

#[cfg(test)]
mod v19_reader_tests {
    use super::*;
    use std::{
        fs,
        path::PathBuf,
        sync::atomic::{AtomicU64, Ordering},
    };

    static NEXT: AtomicU64 = AtomicU64::new(0);

    struct Lab(PathBuf);

    impl Lab {
        fn new() -> Self {
            let path = std::env::temp_dir().join(format!(
                "rce-v19-readers-{}-{}",
                std::process::id(),
                NEXT.fetch_add(1, Ordering::Relaxed)
            ));
            fs::create_dir(&path).unwrap();
            Self(path)
        }

        fn output(&self) -> PathBuf {
            self.0.join("output")
        }
    }

    impl Drop for Lab {
        fn drop(&mut self) {
            fs::remove_dir_all(&self.0).unwrap();
        }
    }

    fn put16(bytes: &mut [u8], offset: usize, value: u16) {
        bytes[offset..offset + 2].copy_from_slice(&value.to_le_bytes());
    }

    fn put32(bytes: &mut [u8], offset: usize, value: u32) {
        bytes[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
    }

    // Same bounded synthetic WDC5 shape used by the existing camera reader tests.
    fn camera_source(ids: &[u32]) -> Vec<u8> {
        let mut out = vec![0; 384];
        out[..4].copy_from_slice(b"WDC5");
        put32(&mut out, 4, 5);
        put32(&mut out, 136, ids.len() as u32);
        put32(&mut out, 140, 5);
        put32(&mut out, 144, 28);
        put32(&mut out, 156, cameras::SOURCE_LAYOUT);
        put32(&mut out, 160, 1);
        put32(&mut out, 164, ids.len() as u32);
        put16(&mut out, 174, u16::MAX);
        put32(&mut out, 176, 5);
        put32(&mut out, 188, 120);
        put32(&mut out, 200, 1);
        put32(&mut out, 212, 384);
        put32(&mut out, 216, ids.len() as u32);
        put32(&mut out, 228, ids.len() as u32 * 4);
        for (field, (offset, count)) in [(0, 3), (96, 1), (128, 1), (160, 1), (192, 1)]
            .into_iter()
            .enumerate()
        {
            let at = 264 + 24 * field;
            put16(&mut out, at, offset);
            put32(&mut out, at + 16, 32);
            put32(&mut out, at + 20, count);
        }
        for &id in ids {
            let at = out.len();
            out.resize(at + 28, 0);
            put32(&mut out, at + 20, id);
        }
        for id in 1..=ids.len() as u32 {
            out.extend_from_slice(&id.to_le_bytes());
        }
        put32(&mut out, 152, cameras::CameraContract::TABLE_HASH);
        out
    }

    fn numeric_table(columns: usize) -> Vec<u8> {
        let header = (0..columns)
            .map(|index| format!("v{index}"))
            .collect::<Vec<_>>()
            .join("\t");
        let first = (0..columns)
            .map(|index| format!("{index}.5"))
            .collect::<Vec<_>>()
            .join("\t");
        let second = vec!["-0"; columns].join("\t");
        format!("ignored\t{header}\r\nduplicate\t{first}\r\nduplicate\t{second}\r\n").into_bytes()
    }

    fn v19_plan() -> Plan {
        Plan::parse(include_str!(
            "../../../tests/contracts/fixtures/forever-product-v19/disabled.tsv"
        ))
        .unwrap()
    }

    #[test]
    fn v19_synthetic_numeric_rows_use_the_catalog_parsers() {
        let mut parsed = 0;
        for table in gt::Table::ALL {
            if let Some(columns) = table.value_columns() {
                gt::parse_numeric(&numeric_table(columns), table).unwrap();
                parsed += 1;
            }
        }
        assert_eq!(parsed, 14);
        assert_eq!(gt::Table::ALL.len(), 20);
    }

    #[test]
    fn v19_uses_the_v11_cumulative_db2_and_camera_read_caps() {
        let v19 = v19_plan();
        let v11 = Plan::parse(include_str!(
            "../../../tests/contracts/fixtures/forever-product-v11/disabled.tsv"
        ))
        .unwrap();
        assert!(v19.is_gt_cameras());
        for total in [0, DB2_TOTAL_BYTES - 1] {
            assert_eq!(db2_read_limit(&v19, total), db2_read_limit(&v11, total));
        }
        assert_eq!(db2_read_limit(&v19, DB2_TOTAL_BYTES - 1).unwrap(), 1);
        assert_eq!(
            db2_read_limit(&v19, DB2_TOTAL_BYTES).unwrap_err().code,
            "product_db2_total_limit"
        );
        for total in [0, cameras::MAX_TOTAL_BYTES - 1] {
            assert_eq!(
                camera_read_limit(&v19, total),
                camera_read_limit(&v11, total)
            );
        }
        assert_eq!(
            camera_read_limit(&v19, cameras::MAX_TOTAL_BYTES - 1).unwrap(),
            1
        );
        assert_eq!(
            camera_read_limit(&v19, cameras::MAX_TOTAL_BYTES)
                .unwrap_err()
                .code,
            "camera_policy_limit"
        );
    }

    #[test]
    fn v19_camera_plan_uses_shared_reader_with_a_synthetic_sealed_source() {
        let plan = v19_plan();
        assert!(plan.is_gt_cameras());
        let source = camera_source(&[0, 11]);
        let lab = Lab::new();
        let context = Context {
            build: plan.build,
            locale: plan.locale,
            components: plan.components,
            source_sha256: [1; 32],
            plan_sha256: integrity::digest(plan.render().as_bytes()),
            producer_sha256: [2; 32],
        };
        let mut run = Run::create(&lab.output(), context, &[], Default::default()).unwrap();
        run.put(Component::Db2, &cameras::source_path(plan.locale), &source)
            .unwrap();
        run.finish_component(Component::Db2).unwrap();

        let selected = camera_plan(&plan, &mut run, &source).unwrap();
        assert_eq!(selected.file_ids(), &[0, 11].into_iter().collect());
    }
}

#[cfg(all(test, target_os = "linux"))]
pub(super) mod v21_navigation_tests {
    use super::*;
    use forever_db2::catalog::{self, FieldType};
    use forever_navigation::mmap;
    use std::{
        fs,
        os::unix::fs::PermissionsExt,
        path::PathBuf,
        sync::atomic::{AtomicU64, Ordering},
    };

    const V21: &str =
        include_str!("../../../tests/contracts/fixtures/forever-product-v21/navigation-map2.tsv");
    static NEXT_LAB: AtomicU64 = AtomicU64::new(0);

    pub(super) struct Lab(PathBuf);
    impl Lab {
        pub(super) fn new() -> Self {
            let path = std::env::temp_dir().join(format!(
                "rce-v21-navigation-{}-{}",
                std::process::id(),
                NEXT_LAB.fetch_add(1, Ordering::Relaxed)
            ));
            fs::create_dir_all(&path).unwrap();
            Self(path)
        }
        fn scratch(&self, name: &str) -> PathBuf {
            let path = self.0.join(name);
            fs::create_dir(&path).unwrap();
            fs::set_permissions(&path, fs::Permissions::from_mode(0o700)).unwrap();
            path
        }
    }
    impl Drop for Lab {
        fn drop(&mut self) {
            fs::remove_dir_all(&self.0).unwrap();
        }
    }

    fn hex16(value: [u8; 16]) -> String {
        value.iter().map(|byte| format!("{byte:02x}")).collect()
    }
    fn put16(bytes: &mut [u8], at: usize, value: u16) {
        bytes[at..at + 2].copy_from_slice(&value.to_le_bytes());
    }
    fn put32(bytes: &mut [u8], at: usize, value: u32) {
        bytes[at..at + 4].copy_from_slice(&value.to_le_bytes());
    }
    fn write_value(bytes: &mut [u8], at: usize, size: usize, value: i64) {
        bytes[at..at + size].copy_from_slice(&value.to_le_bytes()[..size]);
    }

    fn field_width(kind: FieldType) -> usize {
        match kind {
            FieldType::String
            | FieldType::NonLocalizedString
            | FieldType::Int
            | FieldType::Float => 32,
            FieldType::Byte => 8,
            FieldType::Short => 16,
            FieldType::Long => 64,
        }
    }

    fn synthetic_db2(name: &str, row_id: u32, bad_table_hash: bool) -> Vec<u8> {
        let metadata = catalog::resolve_for(catalog::WOWDBDEFS_70450_CATALOG_ID, name)
            .unwrap()
            .unwrap()
            .metadata();
        assert_eq!(metadata.field_count, metadata.file_field_count);
        let fields = &metadata.fields[..metadata.file_field_count];
        let mut offsets = Vec::new();
        let mut bit_offset = 0usize;
        for field in fields {
            offsets.push(bit_offset);
            bit_offset += field_width(field.kind) * usize::from(field.array_size);
        }
        let record_size = bit_offset.div_ceil(8).max(8);
        let mut record = vec![0; record_size];
        for (field_index, field) in fields.iter().enumerate() {
            if matches!(
                field.kind,
                FieldType::String | FieldType::NonLocalizedString
            ) {
                for array_index in 0..usize::from(field.array_size) {
                    let at = offsets[field_index] / 8 + array_index * 4;
                    put32(&mut record, at, u32::try_from(record_size - at).unwrap());
                }
            }
        }
        if name == "Map" {
            for (field, value) in [(13, -1), (14, -1), (21, 100)] {
                let at = offsets[field] / 8;
                let size = field_width(fields[field].kind) / 8;
                write_value(&mut record, at, size, value);
            }
        }

        const HEADER: usize = 204;
        const SECTION: usize = 40;
        let section_at = HEADER;
        let column_meta_at = HEADER + SECTION + fields.len() * 4;
        let record_at = column_meta_at + fields.len() * 24;
        let string_table = b"synthetic\0";
        let id_table_bytes = if metadata.index < 0 { 4 } else { 0 };
        let mut bytes = vec![0; record_at];
        bytes[..4].copy_from_slice(b"WDC5");
        put32(&mut bytes, 4, 5);
        put32(&mut bytes, 136, 1);
        put32(&mut bytes, 140, fields.len() as u32);
        put32(&mut bytes, 144, record_size as u32);
        put32(&mut bytes, 148, string_table.len() as u32);
        put32(
            &mut bytes,
            152,
            metadata.table_hash.unwrap_or(0) ^ u32::from(bad_table_hash),
        );
        put32(&mut bytes, 156, metadata.layout_hash);
        put32(&mut bytes, 160, 0);
        put32(&mut bytes, 164, 1000);
        bytes[174..176].copy_from_slice(&metadata.index.to_le_bytes());
        put32(&mut bytes, 176, metadata.field_count as u32);
        put32(&mut bytes, 188, (fields.len() * 24) as u32);
        put32(&mut bytes, 200, 1);
        put32(&mut bytes, section_at + 8, record_at as u32);
        put32(&mut bytes, section_at + 12, 1);
        put32(&mut bytes, section_at + 16, string_table.len() as u32);
        put32(&mut bytes, section_at + 24, id_table_bytes as u32);
        for (field_index, field) in fields.iter().enumerate() {
            let at = column_meta_at + field_index * 24;
            put16(&mut bytes, at, offsets[field_index] as u16);
            put32(&mut bytes, at + 16, field_width(field.kind) as u32);
            put32(&mut bytes, at + 20, u32::from(field.array_size));
        }
        bytes.extend_from_slice(&record);
        bytes.extend_from_slice(string_table);
        if metadata.index < 0 {
            bytes.extend_from_slice(&row_id.to_le_bytes());
        } else {
            let index = usize::try_from(metadata.index).unwrap();
            put32(
                &mut bytes[record_at..record_at + record_size],
                offsets[index] / 8,
                row_id,
            );
        }
        bytes
    }

    fn root_file(rows: &[(u32, [u8; 16])]) -> Vec<u8> {
        let mut bytes = b"TSFM".to_vec();
        for value in [20_u32, 2, rows.len() as u32, 0] {
            bytes.extend_from_slice(&value.to_le_bytes());
        }
        for (file_data_id, ckey) in rows {
            for value in [1_u32, 2, 0x1000_0000, 0] {
                bytes.extend_from_slice(&value.to_le_bytes());
            }
            bytes.push(0);
            bytes.extend_from_slice(&file_data_id.to_le_bytes());
            bytes.extend_from_slice(ckey);
        }
        bytes
    }
    fn encoding_file(rows: &[([u8; 16], u32, [u8; 16])]) -> Vec<u8> {
        let mut page = Vec::new();
        for (ckey, size, ekey) in rows {
            page.extend_from_slice(&1_u16.to_le_bytes());
            page.extend_from_slice(&size.to_be_bytes());
            page.extend_from_slice(ckey);
            page.extend_from_slice(ekey);
        }
        assert!(page.len() < 1024);
        page.resize(1024, 0);
        let mut bytes = b"EN\x01\x10\x10".to_vec();
        bytes.extend_from_slice(&1_u16.to_be_bytes());
        bytes.extend_from_slice(&1_u16.to_be_bytes());
        bytes.extend_from_slice(&1_u32.to_be_bytes());
        bytes.extend_from_slice(&0_u32.to_be_bytes());
        bytes.push(0);
        bytes.extend_from_slice(&0_u32.to_be_bytes());
        bytes.extend_from_slice(&rows[0].0);
        bytes.extend_from_slice(&v21_test_md5::digest(&page));
        bytes.extend_from_slice(&page);
        bytes
    }
    fn index_entry(key: [u8; 16], offset: u64, size: u32) -> [u8; 18] {
        let mut row = [0; 18];
        row[..9].copy_from_slice(&key[..9]);
        row[9..14].copy_from_slice(&offset.to_be_bytes()[3..]);
        row[14..].copy_from_slice(&size.to_le_bytes());
        row
    }
    fn index_file(bucket: u8, rows: &[[u8; 18]]) -> Vec<u8> {
        let mut bytes = vec![0; 32];
        bytes[..4].copy_from_slice(&16_u32.to_le_bytes());
        bytes[8..10].copy_from_slice(&7_u16.to_le_bytes());
        bytes[10] = bucket;
        bytes[12..16].copy_from_slice(&[4, 5, 9, 30]);
        let hash = v21_test_jenkins::hashlittle(&bytes[8..24], 0);
        bytes[4..8].copy_from_slice(&hash.to_le_bytes());
        bytes.extend_from_slice(&u32::try_from(rows.len() * 18).unwrap().to_le_bytes());
        let mut checksum = (0, 0);
        for row in rows {
            checksum = v21_test_jenkins::hashlittle2(row, checksum.0, checksum.1);
        }
        bytes.extend_from_slice(&checksum.0.to_le_bytes());
        for row in rows {
            bytes.extend_from_slice(row);
        }
        bytes
    }

    #[derive(Clone, Copy)]
    enum MapMode {
        Normal,
        Encrypted,
        Missing,
    }
    struct CascFixture {
        path: PathBuf,
    }
    impl CascFixture {
        fn new(
            lab: &Lab,
            map_mode: MapMode,
            bad_map_hash: bool,
            bad_liquid_hash: bool,
            extra_files: &[(u32, Vec<u8>)],
        ) -> Self {
            let map_table = catalog::resolve_for(catalog::WOWDBDEFS_70450_CATALOG_ID, "Map")
                .unwrap()
                .unwrap();
            let liquid_table =
                catalog::resolve_for(catalog::WOWDBDEFS_70450_CATALOG_ID, "LiquidType")
                    .unwrap()
                    .unwrap();
            let map_file_data_id = map_table.metadata().file_data_id;
            let liquid_file_data_id = liquid_table.metadata().file_data_id;
            let map_bytes = synthetic_db2("Map", 2, bad_map_hash);
            let liquid_bytes = synthetic_db2("LiquidType", 7, bad_liquid_hash);
            let map_ckey = v21_test_md5::digest(&map_bytes);
            let liquid_ckey = v21_test_md5::digest(&liquid_bytes);
            const MAP_EKEY: [u8; 16] = [0x55; 16];
            const LIQUID_EKEY: [u8; 16] = [0x66; 16];

            let mut roots = vec![(liquid_file_data_id, liquid_ckey)];
            let mut routes = vec![(liquid_ckey, liquid_bytes.len() as u32, LIQUID_EKEY)];
            let mut records = vec![(LIQUID_EKEY, normal_record(&liquid_bytes))];
            if !matches!(map_mode, MapMode::Missing) {
                roots.push((map_file_data_id, map_ckey));
                routes.push((map_ckey, map_bytes.len() as u32, MAP_EKEY));
                let record = match map_mode {
                    MapMode::Normal => normal_record(&map_bytes),
                    MapMode::Encrypted => encrypted_record(),
                    MapMode::Missing => unreachable!(),
                };
                records.push((MAP_EKEY, record));
            }
            for (index, (file_data_id, contents)) in extra_files.iter().enumerate() {
                let ckey = v21_test_md5::digest(contents);
                let ekey = [0x80 + index as u8; 16];
                roots.push((*file_data_id, ckey));
                routes.push((ckey, contents.len() as u32, ekey));
                records.push((ekey, normal_record(contents)));
            }

            const ROOT_EKEY: [u8; 16] = [0x33; 16];
            const ENCODING_EKEY: [u8; 16] = [0x44; 16];
            let root = root_file(&roots);
            let root_ckey = v21_test_md5::digest(&root);
            let mut all_routes = vec![(root_ckey, root.len() as u32, ROOT_EKEY)];
            all_routes.extend(routes);
            let encoding = encoding_file(&all_routes);
            let encoding_ckey = v21_test_md5::digest(&encoding);
            records.push((ROOT_EKEY, normal_record(&root)));
            records.push((ENCODING_EKEY, normal_record(&encoding)));

            let config = format!(
                "root = {}\nencoding = {} {}\nencoding-size = {} {}\nbuild-uid = wow_classic_beta\nbuild-name = synthetic-1.60.1.70450\n",
                hex16(root_ckey),
                hex16(encoding_ckey),
                hex16(ENCODING_EKEY),
                encoding.len(),
                encoding.len() + 39
            )
            .into_bytes();
            let config_ckey = v21_test_md5::digest(&config);
            let config_name = hex16(config_ckey);
            let path = lab.0.join(format!(
                "client-{}",
                NEXT_LAB.fetch_add(1, Ordering::Relaxed)
            ));
            let data = path.join("Data/data");
            let config_dir = path
                .join("Data/config")
                .join(&config_name[..2])
                .join(&config_name[2..4]);
            fs::create_dir_all(&data).unwrap();
            fs::create_dir_all(&config_dir).unwrap();
            fs::write(config_dir.join(&config_name), &config).unwrap();
            fs::write(
                path.join(".build.info"),
                format!(
                    "Product!STRING:0|Active!DEC:1|Version!STRING:0|Build Key!HEX:16|CDN Key!HEX:16|Tags!STRING:0\nwow_classic_beta|1|1.60.1.70450|{}|{}|enUS\n",
                    hex16(config_ckey),
                    hex16([0x21; 16])
                ),
            )
            .unwrap();
            let mut archive = Vec::new();
            let mut index_rows = Vec::new();
            for (ekey, record) in records {
                let offset = archive.len() as u64;
                archive.extend_from_slice(&[0xa5; 30]);
                archive.extend_from_slice(&record);
                index_rows.push(index_entry(
                    ekey,
                    offset,
                    u32::try_from(30 + record.len()).unwrap(),
                ));
            }
            index_rows.sort_unstable_by_key(|row| row[..9].to_vec());
            fs::write(data.join("data.000"), archive).unwrap();
            for bucket in 0..16_u8 {
                let rows = if bucket == 0 {
                    index_rows.clone()
                } else {
                    vec![index_entry([0xe0 + bucket; 16], 0, 39)]
                };
                fs::write(
                    data.join(format!("{bucket:02x}00000001.idx")),
                    index_file(bucket, &rows),
                )
                .unwrap();
            }
            Self { path }
        }
    }

    fn normal_record(bytes: &[u8]) -> Vec<u8> {
        let mut record = b"BLTE\0\0\0\0N".to_vec();
        record.extend_from_slice(bytes);
        record
    }
    fn encrypted_record() -> Vec<u8> {
        let mut record = b"BLTE\0\0\0\0E\x08".to_vec();
        record.extend([0x91; 8]);
        record.push(4);
        record.extend([1, 2, 3, 4]);
        record.extend([b'S', 0x72]);
        record
    }

    fn v21_plan() -> Plan {
        Plan::parse(V21).unwrap()
    }

    fn chunk(tag: &[u8; 4], payload: &[u8]) -> Vec<u8> {
        let mut bytes: Vec<_> = tag.iter().rev().copied().collect();
        bytes.extend_from_slice(&(payload.len() as u32).to_le_bytes());
        bytes.extend_from_slice(payload);
        bytes
    }

    fn synthetic_wdt() -> Vec<u8> {
        let mut mphd = vec![0; 32];
        put32(&mut mphd, 0, 0x200);
        let mut main = vec![0; 64 * 64 * 8];
        put32(&mut main, 0, 1);
        let mut maid = vec![0; 64 * 64 * 32];
        put32(&mut maid, 0, 200);
        put32(&mut maid, 4, 201);
        let mut bytes = chunk(b"MVER", &18_u32.to_le_bytes());
        bytes.extend_from_slice(&chunk(b"MPHD", &mphd));
        bytes.extend_from_slice(&chunk(b"MAIN", &main));
        bytes.extend_from_slice(&chunk(b"MAID", &maid));
        bytes
    }

    fn synthetic_adt() -> Vec<u8> {
        let mut header = vec![0; 128];
        put32(&mut header, 52, 42);
        put32(&mut header, 112, 2.0_f32.to_bits());
        let mut heights = vec![0; 145 * 4];
        for index in 0..145 {
            put32(&mut heights, index * 4, (index as f32 * 0.125).to_bits());
        }
        let mut cell = header;
        cell.extend_from_slice(&chunk(b"MCVT", &heights));
        let mut bytes = chunk(b"MVER", &18_u32.to_le_bytes());
        bytes.extend_from_slice(&chunk(b"MCNK", &cell));
        bytes
    }

    fn synthetic_collision_wdt() -> Vec<u8> {
        let mut mphd = vec![0; 32];
        put32(&mut mphd, 0, 0x200);
        let mut main = vec![0; 32768];
        put32(&mut main, 0, 1);
        let mut maid = vec![0; 131072];
        put32(&mut maid, 4, 200);
        let mut bytes = chunk(b"MPHD", &mphd);
        bytes.extend_from_slice(&chunk(b"MAIN", &main));
        bytes.extend_from_slice(&chunk(b"MAID", &maid));
        bytes
    }

    fn synthetic_collision_adt() -> Vec<u8> {
        let mut row = [0; 36];
        put32(&mut row, 0, 300);
        put32(&mut row, 4, 42);
        row[32..34].copy_from_slice(&1024_u16.to_le_bytes());
        row[34..36].copy_from_slice(&0x40_u16.to_le_bytes());
        chunk(b"MDDF", &row)
    }

    fn synthetic_collision_m2() -> Vec<u8> {
        let mut bytes = vec![0; 304];
        bytes[..4].copy_from_slice(b"MD20");
        for (at, value) in [(216, 3), (220, 304), (224, 3), (228, 310)] {
            put32(&mut bytes, at, value);
        }
        bytes.extend_from_slice(&[0, 0, 1, 0, 2, 0]);
        for value in [0.0_f32, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0, 0.0] {
            bytes.extend_from_slice(&value.to_bits().to_le_bytes());
        }
        bytes
    }

    pub(super) fn v22_geometry_package(lab: &Lab) -> PathBuf {
        let table_id = |name: &str| {
            catalog::resolve_for(catalog::WOWDBDEFS_70450_CATALOG_ID, name)
                .unwrap()
                .unwrap()
                .metadata()
                .file_data_id
        };
        let extra_files = [
            (100, synthetic_collision_wdt()),
            (200, synthetic_collision_adt()),
            (300, synthetic_collision_m2()),
            (
                table_id("LiquidMaterial"),
                synthetic_db2("LiquidMaterial", 1, false),
            ),
            (
                table_id("GameObjectDisplayInfo"),
                b"poisonous synthetic GODI: any read must fail".to_vec(),
            ),
            (
                table_id("LiquidObject"),
                b"poisonous synthetic LiquidObject: any read must fail".to_vec(),
            ),
        ];
        let fixture = CascFixture::new(lab, MapMode::Normal, false, false, &extra_files);
        let plan = Plan::parse(include_str!(
            "../../../tests/contracts/fixtures/forever-product-v22/map2.tsv"
        ))
        .unwrap();
        let output = lab.0.join("v22-package");
        let scratch = lab.scratch("v22-scratch");
        extract_disk(
            &plan,
            &fixture.path,
            &output,
            [0x7b; 32],
            false,
            None,
            &scratch,
        )
        .expect("synthetic v22 placement-only extraction");
        output
    }

    #[test]
    fn v21_domain_path_rejects_a_hash_the_pinned_path_accepts() {
        let mut bytes = synthetic_db2("Map", 2, false);
        let original_hash = u32::from_le_bytes(bytes[152..156].try_into().unwrap());
        bytes[152..156].copy_from_slice(&(original_hash ^ 1).to_le_bytes());

        assert!(forever_db2::terrain_sources::read_navigation_maps(&bytes).is_ok());
        let target_error = match forever_db2::terrain_sources::read_navigation_maps_70450(&bytes) {
            Ok(_) => panic!("70450 reader accepted a foreign Map hash"),
            Err(error) => error,
        };
        assert_eq!(target_error.code(), "unsupported_table_schema");
        assert!(crate::map_domain::Domain::derive(&bytes, 2, true).is_ok());
        assert_eq!(
            crate::map_domain::Domain::derive_70450(&bytes, 2, true)
                .err()
                .unwrap()
                .code,
            "unsupported_table_schema"
        );
    }

    #[test]
    fn v21_unavailable_or_encrypted_map_fails_before_creating_package_output() {
        let plan = v21_plan();
        for (mode, expected) in [
            (MapMode::Encrypted, "local_store_no_verified_encoding"),
            (MapMode::Missing, "file_unavailable"),
        ] {
            let lab = Lab::new();
            let fixture = CascFixture::new(&lab, mode, false, false, &[]);
            let output = lab.0.join("package");
            let scratch = lab.scratch("scratch");
            let error = extract_disk(
                &plan,
                &fixture.path,
                &output,
                [0x77; 32],
                false,
                None,
                &scratch,
            )
            .unwrap_err();
            assert_eq!(error.code, expected, "mode {expected}");
            assert!(!output.exists(), "failed domain admission created output");
        }
    }

    #[test]
    fn v21_navigation_producer_uses_the_70450_map_reader() {
        let lab = Lab::new();
        let fixture = CascFixture::new(&lab, MapMode::Normal, true, false, &[]);
        let plan = v21_plan();
        let mut store = open(&plan, &fixture.path).unwrap();
        let output = lab.0.join("package");
        let context = context(&plan, &store, [0x79; 32]).unwrap();
        let mut run = Run::create(&output, context, &[], Default::default()).unwrap();
        let error = mmaps(&plan, &mut store, &output, &mut run, None).unwrap_err();
        assert_eq!(error.code, "unsupported_table_schema");
        assert!(!output.join("mmaps").exists());
    }

    #[test]
    fn v21_terrain_runner_uses_build70450_liquid_material_reader() {
        let lab = Lab::new();
        let mut extra_files = vec![
            (100, synthetic_wdt()),
            (200, synthetic_adt()),
            (201, synthetic_adt()),
        ];
        for name in ["LiquidMaterial", "LiquidObject", "GameObjectDisplayInfo"] {
            let table = catalog::resolve_for(catalog::WOWDBDEFS_70450_CATALOG_ID, name)
                .unwrap()
                .unwrap();
            extra_files.push((
                table.metadata().file_data_id,
                synthetic_db2(name, 1, name == "LiquidMaterial"),
            ));
        }
        let foreign_material = synthetic_db2("LiquidMaterial", 1, true);
        assert!(forever_db2::terrain_sources::read_liquid_materials(&foreign_material).is_ok());
        let target_error =
            match forever_db2::terrain_sources::read_liquid_materials_70450(&foreign_material) {
                Ok(_) => panic!("70450 liquid material reader accepted a foreign hash"),
                Err(error) => error,
            };
        assert_eq!(target_error.code(), "unsupported_table_schema");

        let fixture = CascFixture::new(&lab, MapMode::Normal, false, false, &extra_files);
        let plan = v21_plan();
        let output = lab.0.join("package");
        let scratch = lab.scratch("scratch");
        let error = extract_disk(
            &plan,
            &fixture.path,
            &output,
            [0x7a; 32],
            false,
            None,
            &scratch,
        )
        .unwrap_err();
        assert_eq!(error.code, "unsupported_table_schema");
        assert_eq!(error.component, Some(Component::Terrain));
    }

    #[test]
    fn v21_full_extract_emits_and_verifies_all_components_and_mmap_containers() {
        let lab = Lab::new();
        let mut extra_files = vec![
            (100, synthetic_wdt()),
            (200, synthetic_adt()),
            (201, synthetic_adt()),
        ];
        for name in ["LiquidMaterial", "LiquidObject", "GameObjectDisplayInfo"] {
            let table = catalog::resolve_for(catalog::WOWDBDEFS_70450_CATALOG_ID, name)
                .unwrap()
                .unwrap();
            extra_files.push((table.metadata().file_data_id, synthetic_db2(name, 1, false)));
        }
        let fixture = CascFixture::new(&lab, MapMode::Normal, false, false, &extra_files);
        let plan = v21_plan();
        let output = lab.0.join("package");
        let scratch = lab.scratch("scratch");
        let summary = extract_disk(
            &plan,
            &fixture.path,
            &output,
            [0x79; 32],
            false,
            None,
            &scratch,
        )
        .unwrap();
        assert_eq!(summary.context.components, 57);
        for path in [
            "dbc/enUS/Map.db2",
            "dbc/enUS/LiquidType.db2",
            "maps/0002.tilelist",
            "maps/0002_00_00.map",
            "vmaps/GameObjectModels.dtree",
            "vmaps/0002/0002.vmtree",
            "mmaps/0002.mmap",
            "mmaps/0002_00_00.mmtile",
        ] {
            assert!(output.join(path).is_file(), "missing {path}");
        }

        let global = fs::read(output.join("mmaps/0002.mmap")).unwrap();
        assert_eq!(
            u32::from_le_bytes(global[..4].try_into().unwrap()),
            mmap::MAGIC
        );
        assert_eq!(
            u32::from_le_bytes(global[4..8].try_into().unwrap()),
            mmap::VERSION
        );
        assert_eq!(
            mmap::GlobalView::parse(&global, 2, Default::default())
                .unwrap()
                .sources()
                .count(),
            0
        );

        let tile = fs::read(output.join("mmaps/0002_00_00.mmtile")).unwrap();
        assert_eq!(
            u32::from_le_bytes(tile[..4].try_into().unwrap()),
            mmap::MAGIC
        );
        assert_eq!(
            u32::from_le_bytes(tile[8..12].try_into().unwrap()),
            mmap::VERSION
        );
        let view = mmap::TileView::parse(&tile, Default::default()).unwrap();
        assert!(!view.payload().is_empty());
        assert_eq!(
            tile.len() - 20,
            u32::from_le_bytes(tile[12..16].try_into().unwrap()) as usize
        );

        let verified = crate::verify::verify(&plan, &output, Some(summary.context)).unwrap();
        assert_eq!(verified, summary);
    }
}
