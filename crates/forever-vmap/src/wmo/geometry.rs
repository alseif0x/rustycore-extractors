// SPDX-License-Identifier: GPL-3.0-or-later
// Authored 2026-10-11. Facts: wmo.cpp:443-504,559-572; vmapexport.cpp:255-293.
use super::{Error, Group, Limits, Root, cstring, reserve};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SkipReason {
    Unreachable,
    AntiportalFlag,
    AntiportalName,
}

/// Complete group acquisitions, in the root's nonzero detailed GFID order.
/// Repeated FDIDs retain separate ordinal positions. No completeness assertion
/// is inferred from the name of a file or from the source groupWMOID.
#[derive(Clone, Copy)]
pub struct GroupInput<'g, 'a> {
    pub file_data_id: u32,
    pub group: &'g Group<'a>,
}

pub struct GroupContext<'g, 'a> {
    group: &'g Group<'a>,
    file_data_id: u32,
    ordinal: usize,
    group_wmo_id: u32,
    skip: Option<SkipReason>,
}

/// Resolve the complete ordered inventory before exposing any result. Split
/// inheritance uses the C++ mutable, single forward pass: an earlier parent's
/// changed ID is visible, a later parent's original ID is visible, and a
/// skipped parent's ID is unchanged. Invalid/out-of-range signed split indices
/// keep the original ID. This is not recursive graph resolution.
pub fn resolve_contexts<'g, 'a>(
    root: &Root<'_>,
    groups: &[GroupInput<'g, 'a>],
    limits: Limits,
) -> Result<Vec<GroupContext<'g, 'a>>, Error> {
    limits.validate()?;
    if root.bytes().len() > limits.max_input_bytes
        || root.chunk_count() > limits.max_chunks
        || root.header().groups as usize > limits.max_groups
        || groups.len() > limits.max_groups
    {
        return Err(Error::Limit);
    }
    if root.detailed_group_ids().count() != groups.len() {
        return Err(Error::Inventory);
    }
    for (input, expected) in groups.iter().zip(root.detailed_group_ids()) {
        if input.file_data_id != expected {
            return Err(Error::Inventory);
        }
        if input.group.bytes().len() > limits.max_input_bytes
            || input.group.chunk_count() > limits.max_chunks
        {
            return Err(Error::Limit);
        }
    }
    let mut remaining = limits.max_projection_bytes;
    let mut contexts = reserve::<GroupContext<'g, 'a>>(groups.len(), &mut remaining)?;
    for (ordinal, input) in groups.iter().enumerate() {
        let h = input.group.header();
        let skip = if h.flags & 0x80 != 0 {
            Some(SkipReason::Unreachable)
        } else if h.flags & 0x0400_0000 != 0 {
            Some(SkipReason::AntiportalFlag)
        } else {
            // Negative offsets would cause undefined pointer access in the
            // source. Nonnegative offsets past MOGN are not dereferenced.
            let at = usize::try_from(h.name_offset).map_err(|_| Error::Name)?;
            if at < root.group_names().len() && cstring(root.group_names(), at)? == b"antiportal" {
                Some(SkipReason::AntiportalName)
            } else {
                None
            }
        };
        contexts.push(GroupContext {
            group: input.group,
            file_data_id: input.file_data_id,
            ordinal,
            group_wmo_id: h.group_wmo_id,
            skip,
        });
    }
    for i in 0..contexts.len() {
        let h = contexts[i].group.header();
        if contexts[i].skip.is_none()
            && h.flags2 & 0x80 != 0
            && let Ok(parent) = usize::try_from(h.parent_or_first_child_split_group)
            && let Some(parent) = contexts.get(parent)
        {
            let group_wmo_id = parent.group_wmo_id;
            contexts[i].group_wmo_id = group_wmo_id;
        }
    }
    Ok(contexts)
}

impl<'g, 'a> GroupContext<'g, 'a> {
    pub const fn group(&self) -> &'g Group<'a> {
        self.group
    }
    pub const fn file_data_id(&self) -> u32 {
        self.file_data_id
    }
    pub const fn ordinal(&self) -> usize {
        self.ordinal
    }
    pub const fn group_wmo_id(&self) -> u32 {
        self.group_wmo_id
    }
    pub const fn skip_reason(&self) -> Option<SkipReason> {
        self.skip
    }

    /// Geometry only, not a raw WMO conversion. Liquid and doodad metadata stay
    /// on `group()`. Empty collision geometry is a successful empty projection;
    /// `None` means this group was explicitly skipped by the native predicate.
    pub fn project_geometry(&self, limits: Limits) -> Result<Option<Geometry>, Error> {
        limits.input(self.group.bytes())?;
        let g = self.group;
        if g.vertex_count() > limits.max_vertices
            || g.index_count() > limits.max_indices
            || g.batch_count() > limits.max_metadata_records
            || g.chunk_count() > limits.max_chunks
        {
            return Err(Error::Limit);
        }
        if self.skip.is_some() {
            return Ok(None);
        }
        let mut selected_indices = 0usize;
        for i in 0..g.triangle_count() {
            if keep(g.triangle_material(i).ok_or(Error::Shape)?.flags) {
                selected_indices += 3;
            }
        }
        let mut remaining = limits.max_projection_bytes;
        let mut renumber = reserve::<u32>(g.vertex_count(), &mut remaining)?;
        renumber.resize(g.vertex_count(), u32::MAX);
        for i in 0..g.triangle_count() {
            if keep(g.triangle_material(i).ok_or(Error::Shape)?.flags) {
                for j in 0..3 {
                    let index = g.source_index(i * 3 + j).ok_or(Error::Index)? as usize;
                    *renumber.get_mut(index).ok_or(Error::Index)? = 0;
                }
            }
        }
        let mut selected_vertices = 0;
        for number in &mut renumber {
            if *number != u32::MAX {
                *number = selected_vertices;
                selected_vertices += 1;
            }
        }
        let mut vertices = reserve::<[u32; 3]>(selected_vertices as usize, &mut remaining)?;
        let mut indices = reserve::<u32>(selected_indices, &mut remaining)?;
        let mut batches = reserve::<u32>(g.batch_count(), &mut remaining)?;
        for i in 0..g.batch_count() {
            batches.push(g.batch_value(i).ok_or(Error::Shape)?);
        }
        for (i, number) in renumber.iter().enumerate() {
            if *number != u32::MAX {
                vertices.push(g.vertex_bits(i).ok_or(Error::Shape)?);
            }
        }
        for i in 0..g.triangle_count() {
            if keep(g.triangle_material(i).ok_or(Error::Shape)?.flags) {
                for j in 0..3 {
                    let source = g.source_index(i * 3 + j).ok_or(Error::Index)? as usize;
                    indices.push(*renumber.get(source).ok_or(Error::Index)?);
                }
            }
        }
        Ok(Some(Geometry {
            flags: g.header().flags,
            group_wmo_id: self.group_wmo_id,
            bounds: g.header().bounds_bits,
            vertices,
            indices,
            batches,
        }))
    }
}

fn keep(flags: u16) -> bool {
    flags & 0x08 != 0 || (flags & 0x20 != 0 && flags & 0x04 == 0)
}

/// Owned collision-only geometry. Vertices retain exact f32 bits in original
/// vertex ordinal order; triangle order and winding are unchanged. GRP batch
/// values are not recomputed after collision filtering, matching the pin.
pub struct Geometry {
    flags: u32,
    group_wmo_id: u32,
    bounds: [u32; 6],
    vertices: Vec<[u32; 3]>,
    indices: Vec<u32>,
    batches: Vec<u32>,
}
impl Geometry {
    pub const fn flags(&self) -> u32 {
        self.flags
    }
    pub const fn group_wmo_id(&self) -> u32 {
        self.group_wmo_id
    }
    pub const fn bounds_bits(&self) -> [u32; 6] {
        self.bounds
    }
    pub fn vertices_bits(&self) -> &[[u32; 3]] {
        &self.vertices
    }
    pub fn indices(&self) -> &[u32] {
        &self.indices
    }
    pub fn batches(&self) -> &[u32] {
        &self.batches
    }
    pub fn triangle_count(&self) -> usize {
        self.indices.len() / 3
    }
}
