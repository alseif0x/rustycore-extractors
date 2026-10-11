// SPDX-License-Identifier: GPL-3.0-or-later
// Authored 2026-10-11: immutable slot-order views for geometry consumers.
use super::*;

/// Borrow of one active slot and its exact first-bound placement/model. A slot
/// with a null model (unloaded or excluded path-only record) is never yielded.
/// The store cannot be changed while these borrows exist. No source I/O,
/// allocation, model reparse or geometry copy is performed.
#[derive(Clone, Copy)]
pub struct LoadedModel<'a> {
    slot: u32,
    placement: placement::Placement<'a>,
    geometry: &'a vmo::Owned,
    transform: &'a query::model::Instance,
}
impl<'a> LoadedModel<'a> {
    pub const fn slot(self) -> u32 {
        self.slot
    }
    pub const fn placement(self) -> placement::Placement<'a> {
        self.placement
    }
    pub const fn geometry(self) -> &'a vmo::Owned {
        self.geometry
    }
    pub const fn transform(self) -> &'a query::model::Instance {
        self.transform
    }
}
/// Scans at most the map's declared primitive-slot count, in original tree slot
/// order, independent of load order, spawn IDs, BIH traversal and model-cache
/// order. Cloning only clones the cursor; no storage is allocated.
#[derive(Clone)]
pub struct LoadedModels<'a> {
    slots: std::iter::Enumerate<std::slice::Iter<'a, Slot>>,
    models: &'a [Option<Model>],
}
impl<'a> Iterator for LoadedModels<'a> {
    type Item = LoadedModel<'a>;
    fn next(&mut self) -> Option<Self::Item> {
        for (slot, s) in self.slots.by_ref() {
            let Some(b) = s.binding.as_ref().filter(|b| b.references != 0) else {
                continue;
            };
            let Some(handle) = b.model else { continue };
            let model = self.models[handle]
                .as_ref()
                .expect("active slot owns model");
            // The complete record was admitted by TileView before binding. This
            // allocation-free decode borrows its name and copies numeric bits.
            let record = placement::DirBin::parse(&b.record, placement::Limits::default())
                .expect("validated bound spawn");
            let placement = record.records().next().expect("one bound spawn");
            return Some(LoadedModel {
                slot: slot as u32,
                placement,
                geometry: &model.model,
                transform: &b.instance,
            });
        }
        None
    }
    fn size_hint(&self) -> (usize, Option<usize>) {
        (0, Some(self.slots.len()))
    }
}
impl std::iter::FusedIterator for LoadedModels<'_> {}
impl<S: Source> StaticStore<S> {
    /// Uninitialized declared maps yield an empty iterator. Unknown or poisoned
    /// stores reject. Explicit source verification remains the caller's policy;
    /// this API borrows the already checked immutable in-memory snapshot only.
    pub fn loaded_models(&self, map: u32) -> Result<LoadedModels<'_>, Error> {
        self.healthy()?;
        self.core.parent(map)?;
        let slots = self
            .core
            .map(map)
            .ok()
            .map_or(&[][..], |i| self.core.maps[i].slots.as_slice());
        Ok(LoadedModels {
            slots: slots.iter().enumerate(),
            models: &self.core.models,
        })
    }
}
