// SPDX-License-Identifier: GPL-3.0-or-later
// Authored 2026-10-13: explicit Windows exclusive-append disk-world product.
//! Private archives are fresh per unfinished collision attempt and retained on
//! success/failure. Only common state3 receipts participate in product replay.
//! Guards and free-space observations do not authenticate a filesystem snapshot,
//! reserve disk capacity, promise crash durability or measure process RSS.
use crate::{Error, driver::windows_disk_capacity as capacity, plan::DiskBudget};
use forever_casc::{
    collision::{assemble::staged, gameobjects, inventory_spool, resources, spool},
    local_store::LocalStore,
    package::{Component, Run},
};
use forever_platform::path::Limits as PathLimits;
use forever_platform::windows::{self, ReadDirectory, ReadFile, append, space};
use std::{
    cell::{Cell, RefCell},
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
};

const RUN_ALLOWANCE: usize = 258 * 1024 * 1024;
const GUARD_ALLOWANCE: usize = 16 * 1024 * 1024;
// The source provider has at most 63 ancestors and one bounded path per lease.
// Reserve the external preflight chains, paths, key lease and guard temporaries;
// all six archive/protected capacities are separately charged by Spool itself.
const PREFLIGHT_ALLOWANCE: usize = 4 * 1024 * 1024;
const COMBINED: usize = 2 * 1024 * 1024 * 1024 - RUN_ALLOWANCE - GUARD_ALLOWANCE;
const MAX_NAMESPACE: &str = ".forever-collision-4294967295-18446744073709551615";
static NEXT: AtomicU64 = AtomicU64::new(0);
#[cfg(test)]
mod tests;

pub(crate) fn admit() -> Result<(), Error> {
    append::admit().map_err(|_| "product_disk_unsupported".into())
}
fn path(path: &Path) -> Result<PathBuf, Error> {
    windows::operator_path(path, PathLimits::default()).map_err(|_| "product_disk_path".into())
}
fn directory(path: &Path) -> Result<ReadDirectory, Error> {
    ReadDirectory::open(path, PathLimits::default()).map_err(|_| "product_disk_path".into())
}
fn optional_directory(path: &Path) -> Result<Option<ReadDirectory>, Error> {
    match ReadDirectory::open(path, PathLimits::default()) {
        Ok(directory) => Ok(Some(directory)),
        Err(forever_platform::Error::Missing) => Ok(None),
        Err(_) => Err("product_disk_path".into()),
    }
}
fn available(directory: &ReadDirectory) -> Result<capacity::Available, Error> {
    let value = space::available(directory).map_err(|e| Error::from(e.code()))?;
    Ok(capacity::Available {
        volume: value.volume,
        allocation_unit: value.allocation_unit,
        bytes: value.bytes,
        inodes: value.inodes,
    })
}
fn add(a: usize, b: usize) -> Result<usize, Error> {
    a.checked_add(b).ok_or_else(|| "product_disk_limit".into())
}
fn phase_budget(external: usize) -> Result<usize, Error> {
    COMBINED
        .checked_sub(external)
        .ok_or_else(|| "product_disk_limit".into())
}
fn spool_bytes(owner: &spool::Spool) -> Result<usize, Error> {
    owner
        .stats()
        .map(|stats| stats.descriptor_bytes)
        .map_err(|e| Error::from(e.code()))
}

pub(crate) struct Preflight<'a> {
    budget: DiskBudget,
    scratch_path: PathBuf,
    scratch: ReadDirectory,
    client_path: PathBuf,
    client: ReadDirectory,
    key_path: Option<PathBuf>,
    key: Option<&'a ReadFile>,
    output_path: PathBuf,
    output_parent: ReadDirectory,
    output: RefCell<Option<ReadDirectory>>,
    output_was_present: bool,
    failed: Cell<bool>,
}
impl<'a> Preflight<'a> {
    /// Readonly admission before LocalStore acquisition or common Run mutation.
    /// The supplied key lease was loaded after pure platform/option admission.
    pub(crate) fn open(
        budget: DiskBudget,
        scratch: &Path,
        output: &Path,
        client: &Path,
        key_path: Option<&Path>,
        key: Option<&'a ReadFile>,
    ) -> Result<Self, Error> {
        budget.validate().map_err(Error::from)?;
        admit()?;
        if key_path.is_some() != key.is_some() {
            return Err("product_keys_input_changed".into());
        }
        let scratch_path = path(scratch)?;
        // Reserve all prospective spelling/depth before filesystem observation.
        path(
            &scratch_path
                .join(MAX_NAMESPACE)
                .join("placements")
                .join("records.archive"),
        )?;
        let output_path = path(output)?;
        let client_path = path(client)?;
        let key_path = key_path.map(path).transpose()?;
        let parent = output_path
            .parent()
            .ok_or_else(|| Error::from("product_disk_path"))?;
        let scratch = directory(&scratch_path)?;
        append::private_directory(&scratch).map_err(|_| Error::from("product_disk_privacy"))?;
        let output_parent = directory(parent)?;
        let output = optional_directory(&output_path)?;
        let client = directory(&client_path)?;
        let output_was_present = output.is_some();
        let result = Self {
            budget,
            scratch_path,
            scratch,
            client_path,
            client,
            key_path,
            key,
            output_path,
            output_parent,
            output: RefCell::new(output),
            output_was_present,
            failed: Cell::new(false),
        };
        result.verify_unchanged()?;
        let scratch_space = available(&result.scratch)?;
        let output = result.output.borrow();
        let output_space = available(output.as_ref().unwrap_or(&result.output_parent))?;
        capacity::admit(budget, scratch_space, output_space)?;
        drop(output);
        result.verify_unchanged()?;
        Ok(result)
    }
    fn check(&self) -> Result<(), Error> {
        self.scratch
            .verify_same_object()
            .map_err(|_| Error::from("product_disk_changed"))?;
        self.client
            .verify_same_object()
            .map_err(|_| Error::from("product_disk_changed"))?;
        self.output_parent
            .verify_same_object()
            .map_err(|_| Error::from("product_disk_changed"))?;
        append::private_directory(&self.scratch)
            .map_err(|_| Error::from("product_disk_changed"))?;
        super::verify_keys(self.key)?;
        let keys: Vec<&ReadFile> = self.key.into_iter().collect();
        append::protect_output(&self.scratch_path, &[&self.client], &keys)
            .map_err(|_| Error::from("product_disk_alias"))?;
        append::protect_output(&self.output_path, &[&self.scratch, &self.client], &keys)
            .map_err(|_| Error::from("product_disk_alias"))?;
        let mut output = self.output.borrow_mut();
        if output.is_none() {
            *output = optional_directory(&self.output_path)?;
        }
        if let Some(output) = output.as_ref() {
            output
                .verify_same_object()
                .map_err(|_| Error::from("product_disk_changed"))?;
            if !self.output_was_present
                && output.identity().volume != self.output_parent.identity().volume
            {
                return Err("product_disk_changed".into());
            }
        }
        Ok(())
    }
    pub(crate) fn verify_unchanged(&self) -> Result<(), Error> {
        if self.failed.get() {
            return Err("product_disk_changed".into());
        }
        let result = self.check();
        if result.is_err() {
            self.failed.set(true);
        }
        result
    }
    fn repository(
        &self,
        namespace: &append::Namespace,
        name: &str,
        bytes: u64,
    ) -> Result<spool::Spool, Error> {
        self.verify_unchanged()?;
        namespace
            .guard()
            .map_err(|_| Error::from("product_disk_changed"))?;
        let keys: Vec<&Path> = self.key_path.iter().map(PathBuf::as_path).collect();
        let protected = spool::ProtectedInputs::capture(&self.client_path, &keys)
            .map_err(|e| Error::from(e.code()))?;
        let resources = resources::Profile::DiskWorldV1;
        let mut limits = resources.spool_limits();
        limits.max_disk_bytes =
            usize::try_from(bytes).map_err(|_| Error::from("product_disk_limit"))?;
        spool::Spool::create_with_backend(
            &namespace.path().join(name),
            protected,
            limits,
            spool::Profile::FixedNtfsV1,
            resources,
            spool::BackendProfile::ExclusiveAppendV1,
        )
        .map_err(|e| Error::from(e.code()))
    }
    pub(crate) fn run(&self, store: &mut LocalStore, run: &mut Run) -> Result<(), Error> {
        let result = self.run_inner(store, run);
        let guard = self.verify_unchanged();
        let result = guard.and(result);
        if result.is_err() {
            self.failed.set(true);
        }
        result
    }
    fn run_inner(&self, store: &mut LocalStore, run: &mut Run) -> Result<(), Error> {
        self.verify_unchanged()?;
        let ordinal = NEXT
            .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |n| n.checked_add(1))
            .map_err(|_| Error::from("product_disk_limit"))?;
        let name = format!(".forever-collision-{}-{ordinal}", std::process::id());
        let namespace =
            append::create_namespace(&self.scratch, &name).map_err(|e| Error::from(e.code()))?;
        let namespace_bytes = namespace
            .resident_bytes()
            .map_err(|e| Error::from(e.code()))?;
        if namespace_bytes
            .checked_add(PREFLIGHT_ALLOWANCE)
            .is_none_or(|n| n > GUARD_ALLOWANCE)
        {
            return Err("product_disk_limit".into());
        }
        // All six owners exist before acquisition, with the same Windows
        // backend/resources and independent downward cumulative disk budgets.
        let resources = resources::Profile::DiskWorldV1;
        let raw = self.repository(&namespace, "raw", self.budget.raw_bytes)?;
        let wdt = self.repository(&namespace, "wdt", self.budget.wdt_bytes)?;
        let cache = self.repository(&namespace, "cache", self.budget.cache_bytes)?;
        let placements = self.repository(&namespace, "placements", self.budget.placement_bytes)?;
        let vmo = self.repository(&namespace, "vmo", self.budget.vmo_bytes)?;
        let maps = self.repository(&namespace, "maps", self.budget.map_bytes)?;
        let wdts = inventory_spool::Inventory::new(wdt, resources.inventory_limits())
            .map_err(|e| Error::from(e.code()))?;
        let caches = inventory_spool::Inventory::new(cache, resources.inventory_limits())
            .map_err(|e| Error::from(e.code()))?;
        // Owners not yet transferred into the active phase stay live. Their
        // actual capacities/reservations cannot borrow the active phase budget;
        // once transferred, that phase charges them through its normal ledger.
        let pending_maps = spool_bytes(&maps)?;
        let pending_models = add(spool_bytes(&vmo)?, pending_maps)?;
        let startup_external = add(
            add(
                wdts.stats()
                    .map_err(|e| Error::from(e.code()))?
                    .reserved_heap_bytes,
                caches
                    .stats()
                    .map_err(|e| Error::from(e.code()))?
                    .reserved_heap_bytes,
            )?,
            add(spool_bytes(&placements)?, pending_models)?,
        )?;
        let mut startup_limits = resources.startup_limits();
        startup_limits.collision.max_memory_bytes = startup_limits
            .collision
            .max_memory_bytes
            .min(phase_budget(startup_external)?);
        let startup = gameobjects::spooled::extract_startup_fdid(store, startup_limits, raw)
            .map_err(|e| Error::from(e.code()))?;
        let mut stage_limits = resources.stage_limits();
        stage_limits.world.max_combined_memory_bytes = phase_budget(pending_models)?;
        let staged = staged::extract(startup, wdts, caches, placements, stage_limits)
            .map_err(|e| Error::from(e.code()))?;
        let mut model_limits = resources.model_limits();
        model_limits.max_memory_bytes = phase_budget(pending_maps)?;
        model_limits.max_output_bytes = model_limits.max_output_bytes.min(
            usize::try_from(self.budget.output_bytes)
                .map_err(|_| Error::from("product_disk_limit"))?,
        );
        let prepared = staged::models::prepare(staged, vmo, model_limits)
            .map_err(|e| Error::from(e.code()))?;
        let mut limits = resources.package_limits();
        limits.max_memory_bytes = COMBINED;
        limits.max_files = 262_144;
        limits.max_output_bytes = usize::try_from(self.budget.output_bytes)
            .map_err(|_| Error::from("product_disk_limit"))?;
        let mut completed =
            staged::package::assemble(prepared, maps, limits).map_err(|e| Error::from(e.code()))?;
        self.verify_unchanged()?;
        namespace
            .guard()
            .map_err(|_| Error::from("product_disk_changed"))?;
        // Complete assembly precedes the first collision put. Common Run's
        // metadata/replay/transient storage is reserved outside COMBINED.
        completed
            .visit(0, |artifact| {
                self.verify_unchanged()?;
                namespace
                    .guard()
                    .map_err(|_| Error::from("product_disk_changed"))?;
                run.put(
                    Component::Collision,
                    &format!("vmaps/{}", artifact.name()),
                    artifact.bytes(),
                )?;
                self.verify_unchanged()?;
                namespace
                    .guard()
                    .map_err(|_| Error::from("product_disk_changed"))?;
                Ok::<(), Error>(())
            })
            .map_err(|failure| {
                let code = failure.code();
                match failure.cause {
                    staged::package::Cause::Callback(error)
                        if code == staged::package::Kind::Callback.code() =>
                    {
                        error
                    }
                    _ => code.into(),
                }
            })?;
        completed
            .verify_unchanged()
            .map_err(|e| Error::from(e.code()))?;
        namespace
            .guard()
            .map_err(|_| Error::from("product_disk_changed"))?;
        self.verify_unchanged()
    }
}
