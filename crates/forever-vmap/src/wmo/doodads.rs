// SPDX-License-Identifier: GPL-3.0-or-later
// Authored 2026-10-11: original pure WMO doodad dependency/selection logic.
// Facts: pin 02245dcd245e7433e524577656177723d3e4992e, wmo.cpp:77-151,
// vmapexport.cpp:255-310 and model.cpp:211-307. No upstream code imported.
//! Dependency requests and admitted doodad references, without acquisition,
//! naming normalization, transforms, placement IDs or output serialization.

use super::{Error, Group, GroupInput, Limits, Root, reserve, resolve_contexts};

#[derive(Clone, Copy)]
pub enum Target<'a> {
    FileDataId(u32),
    /// Source bytes, without the terminating NUL. Not assumed UTF-8 or a local
    /// path. Empty names are requests too, as in the source loop.
    Name(&'a [u8]),
}

#[derive(Clone, Copy)]
pub struct Request<'a> {
    /// MODN byte offset or MODI ordinal, matching MODD's 24-bit NameIndex.
    pub name_index: u32,
    pub target: Target<'a>,
}

/// Requests in physical source order, including names unused by every MODD
/// or set. MODI zero entries alone are omitted. No sensitive Debug output.
pub struct Requests<'r, 'a> {
    root: &'r Root<'a>,
    next: usize,
}

pub fn requests<'r, 'a>(root: &'r Root<'a>) -> Requests<'r, 'a> {
    Requests { root, next: 0 }
}

impl<'a> Iterator for Requests<'_, 'a> {
    type Item = Request<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        if let Some(names) = self.root.doodad_name_bytes() {
            let at = self.next;
            let tail = names.get(at..)?;
            let length = tail.iter().position(|byte| *byte == 0)?;
            self.next = at.checked_add(length)?.checked_add(1)?;
            Some(Request {
                name_index: u32::try_from(at).ok()?,
                target: Target::Name(&tail[..length]),
            })
        } else {
            while self.next < self.root.doodad_file_data_id_count() {
                let at = self.next;
                self.next += 1;
                let id = self.root.doodad_file_data_id(at)?;
                if id != 0 {
                    return Some(Request {
                        name_index: u32::try_from(at).ok()?,
                        target: Target::FileDataId(id),
                    });
                }
            }
            None
        }
    }
}

impl std::iter::FusedIterator for Requests<'_, '_> {}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Outcome {
    Ok,
    OkNoCollision,
    FailedNoCollision,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SetReference {
    pub set_index: usize,
    /// MODD spawn ordinal, not a request ordinal or a generated placement ID.
    pub reference: u16,
}

#[derive(Clone, Copy)]
struct SetRange {
    start: u32,
    end: u32,
}

/// Owned selection metadata only: no roots, source buffers, paths or transforms
/// are retained. References are sorted and unique; the source uses an unordered
/// set, so this deterministic order is not a portable native byte-order claim.
pub struct Selection {
    references: Vec<u16>,
    sets: Vec<SetRange>,
}

impl Selection {
    pub fn references(&self) -> &[u16] {
        &self.references
    }
    pub fn set_count(&self) -> usize {
        self.sets.len()
    }

    /// Retained vector capacities, excluding stack fields/allocator overhead.
    /// Both capacities were checked under the resolve operation's live budget.
    pub fn resident_bytes(&self) -> usize {
        self.references.capacity() * std::mem::size_of::<u16>()
            + self.sets.capacity() * std::mem::size_of::<SetRange>()
    }

    /// Matches the native final extraction state, given the separately proven
    /// collision result. Raw M2 parse `None` must never populate valid names.
    /// Empty geometry succeeds only with a nonempty set table AND reference set;
    /// the references need not intersect a specific requested set at this stage.
    pub fn outcome(&self, has_geometry: bool) -> Outcome {
        if has_geometry {
            Outcome::Ok
        } else if !self.sets.is_empty() && !self.references.is_empty() {
            Outcome::OkNoCollision
        } else {
            Outcome::FailedNoCollision
        }
    }

    /// Set zero first, then the requested set only if nonzero and in range.
    /// Each set traverses sorted references. Overlapping sets retain repeated
    /// occurrences without allocating/copying a second reference vector.
    /// Placement ID assignment and its counter limits belong to a later writer.
    pub fn set_references(&self, requested_set: u16) -> impl Iterator<Item = SetReference> + '_ {
        let extra = usize::from(requested_set);
        let selected = [
            (!self.sets.is_empty()).then_some(0),
            (extra != 0 && extra < self.sets.len()).then_some(extra),
        ];
        selected.into_iter().flatten().flat_map(move |set_index| {
            let range = self.sets[set_index];
            let start = self
                .references
                .partition_point(|value| u32::from(*value) < range.start);
            let end = self
                .references
                .partition_point(|value| u32::from(*value) < range.end);
            self.references[start..end]
                .iter()
                .map(move |reference| SetReference {
                    set_index,
                    reference: *reference,
                })
        })
    }
}

/// Resolve the complete group inventory and caller-confirmed model successes.
/// Callers supply only request indices whose M2 acquisition and raw collision
/// conversion succeeded. No acquisition result is inferred by this module.
///
/// Caller indices may be unordered/repeated (native validity is a set); they
/// are sorted/deduplicated. An unknown index, MODN interior byte offset, or MODI
/// zero slot fails with Error::Name. Membership is sort/merge plus binary search,
/// not an all-records nested scan. Every source is reparsed under caller limits.
///
/// The max_projection_bytes ceiling includes concurrent context, validated-name,
/// seen-reference, result-reference and set-range vector capacities. It excludes
/// borrowed input/caller memory and allocator overhead. No partial selection is
/// returned on failure. MODR references outside MODD are deliberately ignored.
pub fn resolve(
    root: &Root<'_>,
    groups: &[GroupInput<'_, '_>],
    valid_name_indices: &[u32],
    limits: Limits,
) -> Result<Selection, Error> {
    Root::parse(root.bytes(), limits)?;
    for input in groups {
        Group::parse(input.group.bytes(), limits)?;
    }
    if valid_name_indices.len() > limits.max_metadata_records {
        return Err(Error::Limit);
    }
    let contexts = resolve_contexts(root, groups, limits)?;
    let context_bytes = contexts
        .capacity()
        .checked_mul(std::mem::size_of::<super::GroupContext<'_, '_>>())
        .ok_or(Error::Bounds)?;
    let mut remaining = limits
        .max_projection_bytes
        .checked_sub(context_bytes)
        .ok_or(Error::Limit)?;
    let mut valid = reserve::<u32>(valid_name_indices.len(), &mut remaining)?;
    valid.extend_from_slice(valid_name_indices);
    valid.sort_unstable();
    valid.dedup();
    let mut matched = 0;
    for request in requests(root) {
        let Some(&wanted) = valid.get(matched) else {
            break;
        };
        if wanted < request.name_index {
            return Err(Error::Name);
        }
        if wanted == request.name_index {
            matched += 1;
        }
    }
    if matched != valid.len() {
        return Err(Error::Name);
    }

    let seen_count = root.doodad_spawn_count().min(usize::from(u16::MAX) + 1);
    let mut seen = reserve::<u8>(seen_count, &mut remaining)?;
    seen.resize(seen_count, 0);
    for context in &contexts {
        if context.skip_reason().is_some() {
            continue;
        }
        let group = context.group();
        for i in 0..group.doodad_reference_count() {
            let reference = group.doodad_reference(i).ok_or(Error::Bounds)?;
            let Some(spawn) = root.doodad_spawn(usize::from(reference)) else {
                continue;
            };
            if valid.binary_search(&spawn.name_index).is_ok() {
                *seen.get_mut(usize::from(reference)).ok_or(Error::Bounds)? = 1;
            }
        }
    }
    let count = seen.iter().filter(|value| **value != 0).count();
    let mut references = reserve::<u16>(count, &mut remaining)?;
    for (reference, &present) in seen.iter().enumerate() {
        if present != 0 {
            references.push(u16::try_from(reference).map_err(|_| Error::Bounds)?);
        }
    }
    let mut sets = reserve::<SetRange>(root.doodad_set_count(), &mut remaining)?;
    for i in 0..root.doodad_set_count() {
        let set = root.doodad_set(i).ok_or(Error::Bounds)?;
        sets.push(SetRange {
            start: set.start_index,
            end: set
                .start_index
                .checked_add(set.count)
                .ok_or(Error::Bounds)?,
        });
    }
    Ok(Selection { references, sets })
}

#[cfg(test)]
mod tests;
