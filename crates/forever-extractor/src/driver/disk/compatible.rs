// SPDX-License-Identifier: GPL-3.0-or-later
// Authored by RustyCore Extractors, 2026-10-17.
//! Explicit70425/70450 GNU15-v2: one genuine leased world through publication.
use super::*;
use crate::collision_coverage as coverage;
use forever_casc::{
    collision::{compatible::startup, plan::MapSelection},
    integrity,
    package::Context,
};
use staged::{
    compatible as placement,
    models::{compatible as models, package::compatible as package},
};
fn failure(e: package::Error<Error>) -> Error {
    let code = e.code();
    match e.cause {
        package::Cause::Callback(e) if code == staged::package::Kind::Callback.code() => e,
        _ => code.into(),
    }
}
fn put_collision(run: &mut Run, sealed: bool, path: &str, bytes: &[u8]) -> Result<(), Error> {
    if sealed {
        run.verify_sealed_artifact(&forever_casc::package::Artifact {
            component: Component::Collision,
            path: path.to_owned(),
            bytes: bytes.len() as u64,
            sha256: integrity::digest(bytes),
        })?;
    } else {
        run.put(Component::Collision, path, bytes)?;
    }
    Ok(())
}
impl Preflight {
    pub(in crate::driver) fn run_compatible_70425(
        &self,
        store: &mut LocalStore,
        run: &mut Run,
        selected: Option<u32>,
        context: Context,
    ) -> Result<(), Error> {
        self.run_compatible(store, run, selected, context, 70425)
    }
    pub(in crate::driver) fn run_compatible_70450(
        &self,
        store: &mut LocalStore,
        run: &mut Run,
        selected: Option<u32>,
        context: Context,
    ) -> Result<(), Error> {
        self.run_compatible(store, run, selected, context, 70450)
    }
    fn run_compatible(
        &self,
        store: &mut LocalStore,
        run: &mut Run,
        selected: Option<u32>,
        context: Context,
        expected_build: u32,
    ) -> Result<(), Error> {
        if context.build != expected_build {
            return Err("product_build_mismatch".into());
        }
        let result = self.compatible_inner(store, run, selected, context);
        // All transient/final compatible owners have dropped before these last
        // independent guards, including on allocation/setup/operation failure.
        let source = store.verify_unchanged().map_err(|e| Error::from(e.code()));
        let guard = self.verify_unchanged();
        let priority = result.as_ref().err().is_some_and(|e| {
            e.code == forever_casc::local_store::Error::InputChanged.code()
                || e.code == forever_casc::collision::payload::Error::Changed.code()
        });
        let result = if source.is_err() {
            source
        } else if priority {
            result
        } else {
            guard.and(result)
        };
        if result.is_err() {
            self.failed.set(true);
        }
        result
    }
    fn compatible_inner(
        &self,
        store: &mut LocalStore,
        run: &mut Run,
        selected: Option<u32>,
        context: Context,
    ) -> Result<(), Error> {
        // A sealed resume still rebuilds the genuine six-owner result, then
        // compares every artifact against the immutable published component.
        let sealed = run.completed_components()? & Component::Collision.bit() != 0;
        // Reserved before the first startup read; all subsequent phases receive
        // the reduced ceiling. Common Run and guard charges remain disjoint.
        let combined = COMBINED
            .checked_sub(coverage::HEAP)
            .ok_or("product_disk_limit")?;
        let mut encoded = coverage::Encoder::new()?;
        let mut ids = Vec::new();
        ids.try_reserve_exact(coverage::MAX_MAPS)
            .map_err(|_| "product_allocation")?;
        if ids.capacity() > coverage::MAX_MAPS {
            return Err("product_disk_limit".into());
        }
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
        let mut limits = resources.startup_limits();
        limits.collision.max_memory_bytes = limits.collision.max_memory_bytes.min(combined);
        let startup = match context.build {
            70425 => startup::extract_startup_fdid_native_gnu15_70425(store, limits, raw),
            70450 => startup::extract_startup_fdid_native_gnu15_70450(store, limits, raw),
            _ => return Err("product_build_mismatch".into()),
        }
        .map_err(|e| Error::from(e.code()))?;
        let old = resources.stage_limits();
        let limits = placement::Limits {
            world: forever_casc::collision::compatible::world::inventory::Limits {
                world: old.world.world,
                max_combined_memory_bytes: combined,
            },
            max_heap_bytes: old.max_heap_bytes,
            max_callbacks: old.max_callbacks,
            max_records: old.max_records,
            records: old.records,
        };
        let stage = placement::extract_selected_with_coverage(
            startup,
            wdts,
            caches,
            placements,
            selected.map_or(MapSelection::All, MapSelection::MapAndAncestors),
            limits,
        )
        .map_err(|e| Error::from(e.code()))?;
        let mut limits = resources.model_limits();
        limits.max_memory_bytes = limits.max_memory_bytes.min(combined);
        limits.max_output_bytes = limits
            .max_output_bytes
            .min(usize::try_from(self.budget.output_bytes).map_err(|_| "product_disk_limit")?);
        let models = models::prepare(stage, vmo, limits).map_err(|e| Error::from(e.code()))?;
        let mut limits = resources.package_limits();
        limits.max_memory_bytes = limits.max_memory_bytes.min(combined);
        limits.max_files = limits.max_files.min(262144);
        limits.max_output_bytes = limits
            .max_output_bytes
            .min(usize::try_from(self.budget.output_bytes).map_err(|_| "product_disk_limit")?);
        let mut done =
            package::assemble(models, maps, limits).map_err(|e| Error::from(e.code()))?;
        done.with_coverage(0,|view|{encoded.capture(context,selected,view)?;for m in view.maps().iter().filter(|m|m.wdt()==forever_casc::collision::run::compatible_inventory::coverage::Wdt::Present){if ids.len()==ids.capacity(){return Err("product_disk_limit".into());}ids.push(m.id());}Ok::<_,Error>(())}).map_err(failure)?;
        // First traversal records the complete final inventory before any put.
        // Proofs authenticate complete ID order; they are not private envelopes.
        for &id in &ids {
            done.with_map(id, 0, |m| {
                let b = m.ordered_dir_bin();
                encoded.artifact(
                    &coverage::proof_path(id),
                    b.len() + 16,
                    coverage::proof_digest(id, b)?,
                )
            })
            .map_err(failure)?;
        }
        done.visit(0, |a| {
            encoded.artifact(
                &format!("vmaps/{}", a.name()),
                a.bytes().len(),
                integrity::digest(a.bytes()),
            )
        })
        .map_err(failure)?;
        encoded.finish()?;
        let (files, bytes) = encoded.totals();
        if files > 262144 || bytes > self.budget.output_bytes {
            return Err("product_disk_limit".into());
        }
        for &id in &ids {
            done.with_map(id, coverage::MAX_BYTES, |m| {
                let raw = m.ordered_dir_bin();
                let size = raw.len().checked_add(16).ok_or("product_disk_limit")?;
                if size > coverage::MAX_BYTES {
                    return Err("product_disk_limit".into());
                }
                let mut b = Vec::new();
                b.try_reserve_exact(size)
                    .map_err(|_| "product_allocation")?;
                if b.capacity() > size {
                    return Err("product_disk_limit".into());
                }
                b.extend_from_slice(&coverage::proof_header(id, raw.len())?);
                b.extend_from_slice(raw);
                self.verify_unchanged()?;
                namespace.check()?;
                put_collision(run, sealed, &coverage::proof_path(id), &b)?;
                self.verify_unchanged()?;
                namespace.check()?;
                Ok::<_, Error>(())
            })
            .map_err(failure)?;
        }
        done.visit(0, |a| {
            self.verify_unchanged()?;
            namespace.check()?;
            put_collision(run, sealed, &format!("vmaps/{}", a.name()), a.bytes())?;
            self.verify_unchanged()?;
            namespace.check()?;
            Ok::<_, Error>(())
        })
        .map_err(failure)?;
        self.verify_unchanged()?;
        namespace.check()?;
        put_collision(run, sealed, coverage::PATH, encoded.finish()?)?;
        done.verify_unchanged().map_err(|e| Error::from(e.code()))?;
        namespace.check()?;
        self.verify_unchanged()
    }
}
#[cfg(all(
    test,
    target_arch = "x86_64",
    target_env = "gnu",
    target_pointer_width = "64"
))]
mod tests;
