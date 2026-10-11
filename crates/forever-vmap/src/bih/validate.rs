// SPDX-License-Identifier: GPL-3.0-or-later
// Authored 2026-10-11: complete bounded BIH wire/graph/permutation validation.
use super::*;

pub(super) fn parse(bytes: &[u8], expected: usize, limits: Limits) -> Result<View<'_>, Error> {
    limits.validate()?;
    if bytes.len() > limits.max_bytes || expected > limits.max_primitives || limits.max_depth == 0 {
        return Err(Error::Limit);
    }
    let mut bounds = [0; 6];
    for (axis, bits) in bounds.iter_mut().enumerate() {
        *bits = word(bytes, axis * 4)?;
    }
    let words = word(bytes, 24)? as usize;
    if words > limits.max_tree_words {
        return Err(Error::Limit);
    }
    if words < 3 || !words.is_multiple_of(3) {
        return Err(Error::Framing);
    }
    let tree_end = words
        .checked_mul(4)
        .and_then(|n| n.checked_add(28))
        .ok_or(Error::Framing)?;
    let count = word(bytes, tree_end)? as usize;
    if count > limits.max_primitives {
        return Err(Error::Limit);
    }
    if count != expected {
        return Err(Error::PrimitiveCount);
    }
    let object_start = tree_end.checked_add(4).ok_or(Error::Framing)?;
    let end = count
        .checked_mul(4)
        .and_then(|n| n.checked_add(object_start))
        .ok_or(Error::Framing)?;
    if end != bytes.len() {
        return Err(Error::Framing);
    }
    let tree = bytes.get(28..tree_end).ok_or(Error::Framing)?;
    let objects = bytes.get(object_start..end).ok_or(Error::Framing)?;
    if count == 0 {
        if bounds != [EMPTY_BITS; 6]
            || words != 3
            || word(tree, 0)? != LEAF
            || word(tree, 4)? != 0
            || word(tree, 8)? != 0
        {
            return Err(Error::Empty);
        }
        return Ok(View {
            bytes,
            bounds,
            tree,
            objects,
            stats: ViewStats {
                primitive_count: 0,
                tree_words: 3,
                node_count: 1,
                max_depth: 1,
                validation_peak_bytes: 0,
            },
        });
    }
    Bounds::from_bits(bounds)?;
    let nodes = words / 3;
    let mut memory = Memory::new(limits.max_memory_bytes);
    let mut visited = Vec::new();
    memory.reserve(&mut visited, nodes)?;
    visited.resize(nodes, 0u8);
    let mut permutation = Vec::new();
    memory.reserve(&mut permutation, count)?;
    permutation.resize(count, 0u8);
    for encoded in objects.as_chunks::<4>().0 {
        let id = u32::from_le_bytes(*encoded) as usize;
        let seen = permutation.get_mut(id).ok_or(Error::Permutation)?;
        if *seen != 0 {
            return Err(Error::Permutation);
        }
        *seen = 1;
    }
    // Fixed DFS stack, left before right. Leaf ranges must cover the complete
    // objects array in that source-construction order, without gaps or overlap.
    let mut pending = [(0usize, 0usize); MAX_DEPTH + 1];
    pending[0] = (0, 1);
    let mut used = 1;
    let mut next_slot = 0usize;
    let mut depth_max = 1;
    while used != 0 {
        used -= 1;
        let (node, depth) = pending[used];
        if depth > limits.max_depth {
            return Err(Error::Limit);
        }
        depth_max = depth_max.max(depth);
        if node >= words || !node.is_multiple_of(3) {
            return Err(Error::Graph);
        }
        let mark = &mut visited[node / 3];
        if *mark != 0 {
            return Err(Error::Graph);
        }
        *mark = 1;
        let header = word(tree, node * 4)?;
        let first = word(tree, (node + 1) * 4)?;
        let second = word(tree, (node + 2) * 4)?;
        let axis = (header >> 30) as usize;
        let offset = (header & OFFSET_MASK) as usize;
        let special = header & BVH2 != 0;
        if axis == 3 {
            if special || second != 0 {
                return Err(Error::Graph);
            }
            let length = first as usize;
            let limit = offset.checked_add(length).ok_or(Error::Range)?;
            if length == 0 || offset != next_slot || limit > count {
                return Err(Error::Range);
            }
            next_slot = limit;
            continue;
        }
        let a = f32::from_bits(first);
        let b = f32::from_bits(second);
        let finite_clip = |v: f32| {
            v.is_finite()
                && v >= f32::from_bits(bounds[axis])
                && v <= f32::from_bits(bounds[axis + 3])
        };
        let (left, right) = if special {
            if !finite_clip(a) || !finite_clip(b) || a > b {
                return Err(Error::Clip);
            }
            (true, false)
        } else {
            let left = if first == f32::NEG_INFINITY.to_bits() {
                false
            } else if finite_clip(a) {
                true
            } else {
                return Err(Error::Clip);
            };
            let right = if second == f32::INFINITY.to_bits() {
                false
            } else if finite_clip(b) {
                true
            } else {
                return Err(Error::Clip);
            };
            if !left && !right {
                return Err(Error::Clip);
            }
            (left, right)
        };
        let mut push = |child: usize| -> Result<(), Error> {
            // Only reachable edges are checked. A dead one-child offset can
            // point backwards, to a different live node, or beyond the tree.
            if child <= node || child >= words || !child.is_multiple_of(3) {
                return Err(Error::Graph);
            }
            if used == pending.len() {
                return Err(Error::Limit);
            }
            pending[used] = (child, depth + 1);
            used += 1;
            Ok(())
        };
        if right {
            push(offset.checked_add(3).ok_or(Error::Graph)?)?;
        }
        if left {
            push(offset)?;
        }
    }
    if next_slot != count {
        return Err(Error::Range);
    }
    if visited.contains(&0) {
        return Err(Error::Graph);
    }
    Ok(View {
        bytes,
        bounds,
        tree,
        objects,
        stats: ViewStats {
            primitive_count: count,
            tree_words: words,
            node_count: nodes,
            max_depth: depth_max,
            validation_peak_bytes: memory.peak,
        },
    })
}
