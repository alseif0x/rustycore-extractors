// SPDX-License-Identifier: GPL-3.0-or-later
// Authored 2026-10-11: immutable complete map tree storage, no I/O or selfref.
use super::*;

#[cfg(test)]
mod tests;

/// Owns VMAP_4.E/NODE and its intrinsic-count validated BIH without copying the
/// tree tail. The memory cap includes Vec capacity plus simultaneous validation
/// scratch. This validates structure, not binding to subsequently loaded spawns.
pub struct OwnedTree {
    bytes: Vec<u8>,
    layout: bih::Layout,
    validation_peak: usize,
    primitive_count: usize,
}
impl OwnedTree {
    pub fn parse(bytes: Vec<u8>, limits: Limits) -> Result<Self, Error> {
        limits.validate()?;
        let resident = bytes.capacity();
        let scratch = limits
            .max_memory_bytes
            .checked_sub(resident)
            .ok_or(Error::Limit)?;
        let tree = Tree::parse_intrinsic(
            &bytes,
            Limits {
                max_memory_bytes: scratch,
                ..limits
            },
        )?;
        let primitive_count = tree.bih().stats().primitive_count;
        let validation_peak = resident
            .checked_add(tree.bih().stats().validation_peak_bytes)
            .ok_or(Error::Limit)?;
        let layout = bih::Layout::from_view(tree.bih());
        Ok(Self {
            bytes,
            layout,
            validation_peak,
            primitive_count,
        })
    }
    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }
    pub fn into_bytes(self) -> Vec<u8> {
        self.bytes
    }
    /// Constant-time stack view of the same already validated immutable bytes.
    pub fn bih(&self) -> bih::View<'_> {
        self.layout.bind(&self.bytes[12..])
    }
    pub const fn primitive_count(&self) -> usize {
        self.primitive_count
    }
    pub fn resident_bytes(&self) -> usize {
        self.bytes.capacity()
    }
    pub const fn validation_peak_bytes(&self) -> usize {
        self.validation_peak
    }
}
