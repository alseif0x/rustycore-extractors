// SPDX-License-Identifier: GPL-3.0-or-later
// Authored 2026-10-12: original product admission and closed disk collision path.
//! Explicit disk-world production through the common replay/manifest owner.
//! Space observations admit declared maxima; they do not reserve blocks/inodes,
//! bound filesystem journal/CoW metadata, authenticate a snapshot or measure RSS.
//! Every later write/guard failure remains terminal. Private intermediates are
//! retained on success and failure, never reused, resumed or deleted here.
use crate::{Error, plan::DiskBudget};
use forever_casc::{
    collision::{assemble::staged, gameobjects, inventory_spool, resources, spool},
    local_store::LocalStore,
    package::{Component, Run},
};
use forever_platform::linux::{lease, space};
use std::{
    cell::{Cell, RefCell},
    fs::{self, DirBuilder},
    os::unix::fs::DirBuilderExt,
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
};
mod anchors;
mod capacity;
mod compatible;
#[cfg(all(
    test,
    target_arch = "x86_64",
    target_env = "gnu",
    target_pointer_width = "64"
))]
mod tests;
use anchors::{Chain, Namespace};
const RUN_ALLOWANCE: usize = 258 * 1024 * 1024;
// Bounded ancestor chains for preflight and all six repositories, namespace
// guards and conversion callback path storage. Conservative logical allowance.
const GUARD_ALLOWANCE: usize = 16 * 1024 * 1024;
const COMBINED: usize = 2 * 1024 * 1024 * 1024 - RUN_ALLOWANCE - GUARD_ALLOWANCE;
static NEXT: AtomicU64 = AtomicU64::new(0);
// Leave room for both created components before any filesystem observation.
// The concrete owner names are no longer than "placements".
const MAX_NAMESPACE: &str = ".forever-collision-4294967295-18446744073709551615";
fn scratch_shape(scratch: &Path) -> Result<(), Error> {
    anchors::validate(scratch)?;
    anchors::validate(&scratch.join(MAX_NAMESPACE).join("placements"))
}

pub struct Preflight {
    budget: DiskBudget,
    scratch: Chain,
    client: Chain,
    key: Option<Chain>,
    output_parent: Chain,
    output: PathBuf,
    output_chain: RefCell<Option<Chain>>,
    output_was_present: bool,
    namespace: Namespace,
    failed: Cell<bool>,
}
fn disjoint(a: &Chain, b: &Chain) -> Result<(), Error> {
    if a.path().starts_with(b.path())
        || b.path().starts_with(a.path())
        || a.contains_leaf(b)
        || b.contains_leaf(a)
    {
        Err("product_disk_alias".into())
    } else {
        Ok(())
    }
}
impl Preflight {
    /// Must precede opening LocalStore or creating/resuming the common package.
    /// Scratch is an existing 0700 directory owned by this process's effective
    /// UID. Output may be an existing directory or absent leaf of an existing
    /// parent. The capability is GNU x86_64 LP64 only; no fallback is admitted.
    pub fn open(
        budget: DiskBudget,
        scratch: &Path,
        output: &Path,
        client: &Path,
        key: Option<&Path>,
    ) -> Result<Self, Error> {
        budget.validate().map_err(Error::from)?;
        if !space::supported() || !lease::supported() {
            return Err("product_disk_unsupported".into());
        }
        scratch_shape(scratch)?;
        anchors::validate(output)?;
        anchors::validate(client)?;
        if let Some(key) = key {
            anchors::validate(key)?;
        }
        if output.file_name().is_none() {
            return Err("product_disk_path".into());
        }
        let namespace = Namespace::open()?;
        let scratch = Chain::open(scratch, true)?;
        let current_uid = anchors::effective_uid()?;
        if !scratch.private() || scratch.uid() != current_uid {
            return Err("product_disk_privacy".into());
        }
        let output_parent = Chain::open(
            output
                .parent()
                .ok_or_else(|| Error::from("product_disk_path"))?,
            true,
        )?;
        let output_chain = match fs::symlink_metadata(output) {
            Ok(_) => Some(Chain::open(output, true)?),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => None,
            Err(_) => return Err("product_disk_path".into()),
        };
        let scratch_space = space::available(scratch.file()).map_err(|e| Error::from(e.code()))?;
        let destination = output_chain.as_ref().unwrap_or(&output_parent);
        let output_space =
            space::available(destination.file()).map_err(|e| Error::from(e.code()))?;
        capacity::admit(
            budget,
            scratch.device(),
            scratch_space,
            destination.device(),
            output_space,
        )?;
        let client = Chain::open(client, true)?;
        let key = key.map(|path| Chain::open(path, false)).transpose()?;
        disjoint(&scratch, &client)?;
        if let Some(key) = &key {
            disjoint(&scratch, key)?;
        }
        let output_was_present = output_chain.is_some();
        let preflight = Self {
            budget,
            scratch,
            client,
            key,
            output_parent,
            output: output.to_path_buf(),
            output_chain: RefCell::new(output_chain),
            output_was_present,
            namespace,
            failed: Cell::new(false),
        };
        preflight.verify_unchanged()?;
        Ok(preflight)
    }
    fn check(&self) -> Result<(), Error> {
        self.namespace.check()?;
        self.scratch.check()?;
        self.client.check()?;
        self.output_parent.check()?;
        if let Some(key) = &self.key {
            key.check()?;
        }
        disjoint(&self.scratch, &self.client)?;
        if let Some(key) = &self.key {
            disjoint(&self.scratch, key)?;
        }
        for protected in [Some(&self.scratch), Some(&self.client), self.key.as_ref()]
            .into_iter()
            .flatten()
        {
            if self.output.starts_with(protected.path())
                || protected.path().starts_with(&self.output)
                || self.output_parent.contains_leaf(protected)
            {
                return Err("product_disk_alias".into());
            }
        }
        let mut captured = self.output_chain.borrow_mut();
        if captured.is_none() {
            match fs::symlink_metadata(&self.output) {
                Ok(_) => *captured = Some(Chain::open(&self.output, true)?),
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
                Err(_) => return Err("product_disk_changed".into()),
            }
        }
        if let Some(output) = captured.as_ref() {
            output.check()?;
            // An initially absent output cannot appear on an unobserved mount.
            if !self.output_was_present
                && (output.mount() != self.output_parent.mount()
                    || output.device() != self.output_parent.device())
            {
                return Err("product_disk_changed".into());
            }
            disjoint(output, &self.scratch)?;
            disjoint(output, &self.client)?;
            if let Some(key) = &self.key {
                disjoint(output, key)?;
            }
        }
        self.namespace.check()
    }
    /// Recheck stable identities/mounts and namespace. Directory timestamps and
    /// lengths may change through our own writes. Any observed failure poisons
    /// this capability permanently, even if the pathname is later restored.
    /// Free-space maxima are admitted once, not repeatedly reserved.
    pub fn verify_unchanged(&self) -> Result<(), Error> {
        if self.failed.get() {
            return Err("product_disk_changed".into());
        }
        let result = self.check();
        if result.is_err() {
            self.failed.set(true);
        }
        result
    }
    fn protected(&self) -> Result<spool::ProtectedInputs, Error> {
        let keys: Vec<_> = self.key.iter().map(|key| key.path()).collect();
        spool::ProtectedInputs::capture(self.client.path(), &keys)
            .map_err(|e| Error::from(e.code()))
    }
    fn repository(&self, parent: &Chain, name: &str, bytes: u64) -> Result<spool::Spool, Error> {
        self.verify_unchanged()?;
        parent.check()?;
        let resources = resources::Profile::DiskWorldV1;
        let mut limits = resources.spool_limits();
        limits.max_disk_bytes =
            usize::try_from(bytes).map_err(|_| Error::from("product_disk_limit"))?;
        spool::Spool::create_with_backend(
            &parent.path().join(name),
            self.protected()?,
            limits,
            spool::Profile::CapturedVolumesV2,
            resources,
            spool::BackendProfile::LeasedAppendV1,
        )
        .map_err(|e| Error::from(e.code()))
    }
    fn repositories(&self) -> Result<Repositories, Error> {
        self.verify_unchanged()?;
        let ordinal = NEXT
            .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |n| n.checked_add(1))
            .map_err(|_| Error::from("product_disk_limit"))?;
        let name = format!(".forever-collision-{}-{ordinal}", std::process::id());
        let path = self.scratch.path().join(&name);
        anchors::validate(&path)?;
        DirBuilder::new()
            .mode(0o700)
            .create(anchors::anchor(self.scratch.file()).join(&name))
            .map_err(|e| {
                Error::from(if e.kind() == std::io::ErrorKind::AlreadyExists {
                    "product_disk_exists"
                } else {
                    "product_disk_io"
                })
            })?;
        // Retain even a partially constructed namespace; no path cleanup occurs.
        let namespace = Chain::open(&path, true)?;
        if !namespace.private()
            || namespace.uid() != self.scratch.uid()
            || namespace.mount() != self.scratch.mount()
        {
            return Err("product_disk_changed".into());
        }
        let raw = self.repository(&namespace, "raw", self.budget.raw_bytes)?;
        let wdt = self.repository(&namespace, "wdt", self.budget.wdt_bytes)?;
        let cache = self.repository(&namespace, "cache", self.budget.cache_bytes)?;
        let placements = self.repository(&namespace, "placements", self.budget.placement_bytes)?;
        let vmo = self.repository(&namespace, "vmo", self.budget.vmo_bytes)?;
        let maps = self.repository(&namespace, "maps", self.budget.map_bytes)?;
        Ok(Repositories {
            namespace,
            raw,
            wdt,
            cache,
            placements,
            vmo,
            maps,
        })
    }
    /// Build the complete closed inventory before the first common-package put.
    /// Common Run replay remains authoritative; retained scratch is never used
    /// for recovery. Startup, world and all six owners carry the same explicit
    /// resources/volume/backend profiles and downward-only disk budgets.
    pub fn run(&self, store: &mut LocalStore, run: &mut Run) -> Result<(), Error> {
        self.run_with_domain(store, run, None)
    }
    pub(super) fn run_selected(
        &self,
        store: &mut LocalStore,
        run: &mut Run,
        domain: &crate::map_domain::Domain,
    ) -> Result<(), Error> {
        self.run_with_domain(store, run, Some(domain))
    }
    fn run_with_domain(
        &self,
        store: &mut LocalStore,
        run: &mut Run,
        domain: Option<&crate::map_domain::Domain>,
    ) -> Result<(), Error> {
        let result = self.run_inner(store, run, domain);
        let guard = self.verify_unchanged();
        let result = guard.and(result);
        if result.is_err() {
            self.failed.set(true);
        }
        result
    }
    fn run_inner(
        &self,
        store: &mut LocalStore,
        run: &mut Run,
        domain: Option<&crate::map_domain::Domain>,
    ) -> Result<(), Error> {
        let Repositories {
            namespace,
            raw,
            wdt,
            cache,
            placements,
            vmo,
            maps,
        } = self.repositories()?;
        let resources = resources::Profile::DiskWorldV1;
        let wdts = inventory_spool::Inventory::new(wdt, resources.inventory_limits())
            .map_err(|e| Error::from(e.code()))?;
        let caches = inventory_spool::Inventory::new(cache, resources.inventory_limits())
            .map_err(|e| Error::from(e.code()))?;
        let combined = COMBINED
            .checked_sub(domain.map_or(0, |domain| domain.resident_bytes()))
            .ok_or("product_map_domain_limit")?;
        let mut startup_limits = resources.startup_limits();
        if domain.is_some() {
            startup_limits.collision.max_memory_bytes =
                startup_limits.collision.max_memory_bytes.min(combined);
        }
        let startup = gameobjects::spooled::extract_startup_fdid(store, startup_limits, raw)
            .map_err(|e| Error::from(e.code()))?;
        let mut stage_limits = resources.stage_limits();
        stage_limits.world.max_combined_memory_bytes = combined;
        let staged = match domain {
            Some(domain) => staged::extract_selected(
                startup,
                wdts,
                caches,
                placements,
                domain.selection(),
                stage_limits,
            ),
            None => staged::extract(startup, wdts, caches, placements, stage_limits),
        }
        .map_err(|e| Error::from(e.code()))?;
        let mut model_limits = resources.model_limits();
        model_limits.max_memory_bytes = combined;
        model_limits.max_output_bytes = model_limits.max_output_bytes.min(
            usize::try_from(self.budget.output_bytes)
                .map_err(|_| Error::from("product_disk_limit"))?,
        );
        let prepared = staged::models::prepare(staged, vmo, model_limits)
            .map_err(|e| Error::from(e.code()))?;
        let mut package_limits = resources.package_limits();
        package_limits.max_memory_bytes = combined;
        package_limits.max_files = 262_144;
        package_limits.max_output_bytes = usize::try_from(self.budget.output_bytes)
            .map_err(|_| Error::from("product_disk_limit"))?;
        let mut completed = staged::package::assemble(prepared, maps, package_limits)
            .map_err(|e| Error::from(e.code()))?;
        self.verify_unchanged()?;
        namespace.check()?;
        // Run's full metadata/replay ceiling and transient callback storage are
        // reserved outside COMBINED above, avoiding a second reservation here.
        completed
            .visit(0, |artifact| {
                self.verify_unchanged()?;
                namespace.check()?;
                run.put(
                    Component::Collision,
                    &format!("vmaps/{}", artifact.name()),
                    artifact.bytes(),
                )?;
                self.verify_unchanged()?;
                namespace.check()?;
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
        namespace.check()?;
        self.verify_unchanged()
    }
}

struct Repositories {
    namespace: Chain,
    raw: spool::Spool,
    wdt: spool::Spool,
    cache: spool::Spool,
    placements: spool::Spool,
    vmo: spool::Spool,
    maps: spool::Spool,
}
