// SPDX-License-Identifier: GPL-3.0-or-later
// Modified 2026-10-19: add rooted selected partial-coverage inventory discovery.
// Authored 2026-10-19: Linux-only available selected navigation filesystem owner.
//! Rooted filesystem admission for one explicit 70450 map and its effective
//! ancestor closure. The owner retains the available-section plan and cannot
//! expose or promote it as a complete-source inventory.
#![cfg(target_os = "linux")]

use super::*;
use crate::inventory::{self, AvailableSelectedPlan, SelectedClosure};
use forever_db2::availability::{AvailableRecords, SectionCoverage};
use std::{os::unix::ffi::OsStrExt, path::Path};

#[cfg(test)]
mod tests;

/// A rooted package admitted from an explicitly partial 70450 source snapshot.
/// Discovery and each tile read remain bound to the observed roots and entries.
pub struct AvailableSelectedPackage {
    pub(super) plan: AvailableSelectedPlan,
    pub(super) guard: guard::Guard,
    pub(super) lists: Vec<TileListIdentity>,
    pub(super) budget: Budget,
    pub(super) failed: bool,
    failure: Option<Error>,
    metadata_ceiling: u64,
}

impl AvailableSelectedPackage {
    /// Admit one selected map from the 70450 available-section reader. Copies
    /// remain inspection metadata and do not supply direct rows or parents.
    pub fn open_70450(
        maps: &Path,
        vmaps: &Path,
        source: &AvailableRecords<NavigationMapFields>,
        selected_map_id: u32,
        limits: Limits,
    ) -> Result<Self, Error> {
        limits.validate()?;
        let aggregate_heap_limit = limits.heap_bytes.min(limits.plan.heap_bytes);
        let mut closure_limits = limits.plan;
        closure_limits.heap_bytes = aggregate_heap_limit;
        let closure = inventory::prepare_selected_closure(
            source.direct_records(),
            source.copies(),
            selected_map_id,
            closure_limits,
        )
        .map_err(Error::Inventory)?;
        Self::open_prepared(
            maps,
            vmaps,
            closure,
            source.sections(),
            selected_map_id,
            limits,
            aggregate_heap_limit,
        )
    }

    #[cfg(test)]
    fn open_rows_for_test(
        maps: &Path,
        vmaps: &Path,
        rows: &[DirectRecord<NavigationMapFields>],
        copies: &[forever_db2::terrain_sources::RecordCopy],
        sections: &[SectionCoverage],
        selected_map_id: u32,
        limits: Limits,
    ) -> Result<Self, Error> {
        limits.validate()?;
        let aggregate_heap_limit = limits.heap_bytes.min(limits.plan.heap_bytes);
        let mut closure_limits = limits.plan;
        closure_limits.heap_bytes = aggregate_heap_limit;
        let closure =
            inventory::prepare_selected_closure(rows, copies, selected_map_id, closure_limits)
                .map_err(Error::Inventory)?;
        Self::open_prepared(
            maps,
            vmaps,
            closure,
            sections,
            selected_map_id,
            limits,
            aggregate_heap_limit,
        )
    }

    fn open_prepared(
        maps: &Path,
        vmaps: &Path,
        closure: SelectedClosure,
        sections: &[SectionCoverage],
        selected_map_id: u32,
        limits: Limits,
        aggregate_heap_limit: usize,
    ) -> Result<Self, Error> {
        if sections.len() > limits.descriptors {
            return Err(Error::Limit);
        }
        let closure_heap = closure.retained_heap_bytes().map_err(Error::Inventory)?;
        if closure.heap_high_water() > aggregate_heap_limit || closure_heap > aggregate_heap_limit {
            return Err(Error::Limit);
        }
        let mut discovery_limits = limits;
        discovery_limits.heap_bytes = aggregate_heap_limit - closure_heap;
        let mut budget = Budget::new(discovery_limits)?;
        let maps_root = guard::Root::new(maps, &mut budget)?;
        let vmaps_root = guard::Root::new(vmaps, &mut budget)?;
        let mut guard = guard::Guard::new(maps_root, vmaps_root);
        let mut files = Vec::new();
        let mut descriptors = Vec::new();
        let mut lists = Vec::new();
        scan_maps(
            &mut guard,
            &closure,
            &mut budget,
            &mut files,
            &mut descriptors,
            &mut lists,
        )?;
        scan_vmaps(&mut guard, &closure, &mut budget, &mut descriptors)?;
        guard.canonical_order();
        budget.limits.metadata_calls = limits
            .metadata_calls
            .checked_sub(budget.stats.metadata_calls)
            .ok_or(Error::Limit)?;
        budget.used = 0;
        guard.verify(&mut budget)?;

        descriptors.sort_unstable_by_key(|d| match *d {
            Descriptor::Terrain(i) => (files[i].id, 0, 0, 0),
            Descriptor::Map(id) => (id, 1, 0, 0),
            Descriptor::Tile(id, t) => (id, 2, t.row, t.column),
        });
        let mut inputs = Vec::new();
        budget.reserve(&mut inputs, descriptors.len(), limits.descriptors)?;
        for descriptor in &descriptors {
            inputs.push(match *descriptor {
                Descriptor::Terrain(i) => Input::Terrain {
                    map_id: files[i].id,
                    bytes: &files[i].bytes,
                },
                Descriptor::Map(map_id) => Input::CollisionMap { map_id },
                Descriptor::Tile(map_id, tile) => Input::CollisionTile { map_id, tile },
            });
        }
        let external_work = budget.stats.metadata_calls;
        let mut plan = inventory::from_selected_closure(
            closure,
            sections,
            &inputs,
            selected_map_id,
            limits.plan,
            external_work,
            budget.stats.peak_capacity,
            aggregate_heap_limit,
        )
        .map_err(Error::Inventory)?;
        budget.release(&inputs);
        drop(inputs);
        budget.release(&descriptors);
        drop(descriptors);
        for file in &files {
            budget.release(&file.bytes);
        }
        budget.release(&files);
        drop(files);
        lists.sort_unstable_by_key(|list| list.map_id);
        budget.limits.metadata_calls = limits
            .metadata_calls
            .checked_sub(budget.stats.metadata_calls)
            .ok_or(Error::Limit)?;
        budget.used = 0;
        let final_verification_start = budget.stats.metadata_calls;
        guard.verify(&mut budget)?;
        plan.account_preparation_work(
            budget
                .stats
                .metadata_calls
                .checked_sub(final_verification_start)
                .ok_or(Error::Limit)?,
            limits.plan.work,
        )
        .map_err(Error::Inventory)?;
        budget.limits.heap_bytes = aggregate_heap_limit
            .checked_sub(plan.statistics().heap_reservation)
            .ok_or(Error::Limit)?;
        Ok(Self {
            plan,
            guard,
            lists,
            budget,
            failed: false,
            failure: None,
            metadata_ceiling: limits.metadata_calls,
        })
    }

    pub fn plan(&self) -> &AvailableSelectedPlan {
        &self.plan
    }

    pub fn tilelists(&self) -> &[TileListIdentity] {
        &self.lists
    }

    pub const fn statistics(&self) -> Statistics {
        self.budget.stats
    }

    pub const fn failed(&self) -> bool {
        self.failed
    }

    pub(super) fn latch_failure(&mut self, error: Error) {
        if !self.failed {
            self.failed = true;
            self.failure = Some(error);
        }
    }

    pub(super) fn failure_error(&self) -> Error {
        self.failure.unwrap_or(Error::Failed)
    }

    /// Verifies the complete recorded directory inventory. Any observed
    /// replacement permanently latches this owner as failed.
    pub fn verify_unchanged(&mut self) -> Result<(), Error> {
        if self.failed {
            return Err(self.failure_error());
        }
        if let Err(error) = self.begin_metadata_operation() {
            self.latch_failure(error);
            return Err(error);
        }
        let result = self.guard.verify(&mut self.budget);
        if let Err(error) = result {
            self.latch_failure(error);
            return Err(error);
        }
        Ok(())
    }

    pub(super) fn begin_metadata_operation(&mut self) -> Result<(), Error> {
        let remaining = self
            .metadata_ceiling
            .checked_sub(self.budget.stats.metadata_calls)
            .ok_or(Error::Limit)?;
        self.budget.limits.metadata_calls = remaining;
        self.budget.used = 0;
        Ok(())
    }

    pub(super) fn retained_heap_bytes(&self) -> Result<usize, Error> {
        self.budget
            .stats
            .resident_capacity
            .checked_add(self.plan.statistics().heap_reservation)
            .ok_or(Error::Limit)
    }
}

fn descriptor(d: Descriptor, ds: &mut Vec<Descriptor>, b: &mut Budget) -> Result<(), Error> {
    if b.stats.descriptors >= b.limits.descriptors {
        return Err(Error::Limit);
    }
    b.grow(ds, b.limits.descriptors)?;
    ds.push(d);
    b.stats.descriptors += 1;
    Ok(())
}

fn scan_maps(
    guard: &mut guard::Guard,
    closure: &SelectedClosure,
    budget: &mut Budget,
    files: &mut Vec<TileList>,
    descriptors: &mut Vec<Descriptor>,
    lists: &mut Vec<TileListIdentity>,
) -> Result<(), Error> {
    for entry in fs::read_dir(guard.anchored(0)).map_err(|_| Error::Io)? {
        let entry = entry.map_err(|_| Error::Io)?;
        let name = entry.file_name();
        let name = name.as_bytes();
        budget.metadata()?;
        let metadata = fs::symlink_metadata(entry.path()).map_err(|_| Error::InputChanged)?;
        guard.remember(0, name, &metadata, budget)?;
        if !name.ends_with(b".tilelist") {
            continue;
        }
        let id = map_id(&name[..name.len() - b".tilelist".len()])?;
        if !closure.contains_map(id) {
            continue;
        }
        if files.len() >= budget.limits.tilelists {
            return Err(Error::Limit);
        }
        let bytes = guard.read_list(0, name, &metadata, budget)?;
        inventory::validate_tilelist(&bytes, 70450).map_err(Error::Inventory)?;
        let mut hash = hash::Sha256::new();
        hash.update(&bytes);
        let sha256 = hash.finish();
        budget.grow(lists, budget.limits.tilelists)?;
        lists.push(TileListIdentity { map_id: id, sha256 });
        budget.grow(files, budget.limits.tilelists)?;
        let index = files.len();
        files.push(TileList { id, bytes });
        descriptor(Descriptor::Terrain(index), descriptors, budget)?;
        budget.stats.tilelists += 1;
    }
    Ok(())
}

fn scan_vmaps(
    guard: &mut guard::Guard,
    closure: &SelectedClosure,
    budget: &mut Budget,
    descriptors: &mut Vec<Descriptor>,
) -> Result<(), Error> {
    for entry in fs::read_dir(guard.anchored(1)).map_err(|_| Error::Io)? {
        let entry = entry.map_err(|_| Error::Io)?;
        let name = entry.file_name();
        let name = name.as_bytes();
        budget.metadata()?;
        let metadata = fs::symlink_metadata(entry.path()).map_err(|_| Error::InputChanged)?;
        guard.remember(1, name, &metadata, budget)?;
        if name.is_empty() || !name.iter().all(u8::is_ascii_digit) {
            continue;
        }
        let id = map_id(name)?;
        if !closure.contains_map(id) {
            continue;
        }
        if !metadata.is_dir() || metadata.file_type().is_symlink() {
            return Err(Error::Kind);
        }
        let directory = guard::open_dir(&entry.path())?;
        budget.metadata()?;
        let held = directory.metadata().map_err(|_| Error::InputChanged)?;
        if !guard::same(&metadata, &held) {
            return Err(Error::InputChanged);
        }
        descriptor(Descriptor::Map(id), descriptors, budget)?;
        for file in fs::read_dir(guard::directory(&directory)).map_err(|_| Error::Io)? {
            let file = file.map_err(|_| Error::Io)?;
            let file_name = file.file_name();
            let file_name = file_name.as_bytes();
            let mut relative = [0u8; 511];
            let length = name.len() + 1 + file_name.len();
            if length > relative.len() {
                return Err(Error::Limit);
            }
            relative[..name.len()].copy_from_slice(name);
            relative[name.len()] = b'/';
            relative[name.len() + 1..length].copy_from_slice(file_name);
            budget.metadata()?;
            let metadata = fs::symlink_metadata(file.path()).map_err(|_| Error::InputChanged)?;
            guard.remember(1, &relative[..length], &metadata, budget)?;
            if !file_name.ends_with(b".vmtile") {
                continue;
            }
            if !metadata.is_file() || metadata.file_type().is_symlink() {
                return Err(Error::Kind);
            }
            descriptor(
                Descriptor::Tile(id, tile(file_name, id)?),
                descriptors,
                budget,
            )?;
        }
        budget.metadata()?;
        let after = directory.metadata().map_err(|_| Error::InputChanged)?;
        if !guard::same(&held, &after) {
            return Err(Error::InputChanged);
        }
    }
    Ok(())
}
