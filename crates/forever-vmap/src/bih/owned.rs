// SPDX-License-Identifier: GPL-3.0-or-later
// Authored 2026-10-11: immutable owned storage of an already validated BIH.
use super::*;

#[cfg(test)]
mod tests;

/// Offset-only descriptor produced solely from complete successful validation.
/// A binder must retain the exact immutable bytes used to obtain this layout.
#[derive(Clone, Copy)]
pub(crate) struct Layout {
    bounds: [u32; 6],
    tree_end: usize,
    object_start: usize,
    stats: ViewStats,
}
impl Layout {
    pub(crate) fn from_view(view: &View<'_>) -> Self {
        Self {
            bounds: view.bounds,
            tree_end: 28 + view.tree.len(),
            object_start: 32 + view.tree.len(),
            stats: view.stats,
        }
    }
    // No parsing, allocation or geometry work. Only private immutable owners
    // call this with the same complete bytes; the validated extents cannot move.
    pub(crate) fn bind(self, bytes: &[u8]) -> View<'_> {
        View {
            bytes,
            bounds: self.bounds,
            tree: &bytes[28..self.tree_end],
            objects: &bytes[self.object_start..],
            stats: self.stats,
        }
    }
}

/// Complete immutable BIH bytes plus validated offsets. Moving this object
/// never invalidates its views; each new view is reconstructed on the stack.
/// The memory cap includes the input Vec's actual capacity and validation
/// scratch simultaneously. Stack/allocator overhead are excluded, not RSS.
pub struct Owned {
    bytes: Vec<u8>,
    layout: Layout,
    validation_peak: usize,
}
impl Owned {
    pub fn parse(
        bytes: Vec<u8>,
        expected_primitives: usize,
        limits: Limits,
    ) -> Result<Self, Error> {
        limits.validate()?;
        let resident = bytes.capacity();
        let scratch = limits
            .max_memory_bytes
            .checked_sub(resident)
            .ok_or(Error::Limit)?;
        let view = View::parse(
            &bytes,
            expected_primitives,
            Limits {
                max_memory_bytes: scratch,
                ..limits
            },
        )?;
        let validation_peak = resident
            .checked_add(view.stats.validation_peak_bytes)
            .ok_or(Error::Limit)?;
        let layout = Layout::from_view(&view);
        Ok(Self {
            bytes,
            layout,
            validation_peak,
        })
    }
    pub fn parse_intrinsic(bytes: Vec<u8>, limits: Limits) -> Result<Self, Error> {
        let count = intrinsic_count(&bytes, limits)?;
        Self::parse(bytes, count, limits)
    }
    pub fn view(&self) -> View<'_> {
        self.layout.bind(&self.bytes)
    }
    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }
    pub fn into_bytes(self) -> Vec<u8> {
        self.bytes
    }
    pub const fn stats(&self) -> ViewStats {
        self.layout.stats
    }
    pub const fn primitive_count(&self) -> usize {
        self.layout.stats.primitive_count
    }
    pub fn resident_bytes(&self) -> usize {
        self.bytes.capacity()
    }
    /// Peak includes retained byte capacity; ViewStats' peak is scratch only.
    pub const fn validation_peak_bytes(&self) -> usize {
        self.validation_peak
    }
}
