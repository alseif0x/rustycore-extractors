// SPDX-License-Identifier: GPL-3.0-or-later
// Authored 2026-10-11: original bounded implementation of pinned build facts.
use super::*;

#[derive(Clone, Copy)]
struct Box3([f32; 6]);
impl Box3 {
    fn from_bounds(value: Bounds) -> Self {
        Self(value.bits.map(f32::from_bits))
    }
}
#[derive(Clone, Copy)]
struct Frame {
    start: usize,
    end: usize,
    node: usize,
    depth: usize,
    grid: Box3,
    bounds: Box3,
}
struct Builder<'a> {
    primitives: &'a [Bounds],
    objects: Vec<u32>,
    tree: Vec<[u32; 3]>,
    memory: Memory,
    limits: Limits,
    leaf: usize,
    work: u64,
}
pub(super) fn build(primitives: &[Bounds], leaf_size: u32, limits: Limits) -> Result<Bih, Error> {
    limits.validate()?;
    if !matches!(leaf_size, 1 | 3) {
        return Err(Error::LeafSize);
    }
    if primitives.len() > limits.max_primitives || limits.max_depth == 0 {
        return Err(Error::Limit);
    }
    let mut state = Builder {
        primitives,
        objects: Vec::new(),
        tree: Vec::new(),
        memory: Memory::new(limits.max_memory_bytes),
        limits,
        leaf: leaf_size as usize,
        work: 0,
    };
    state.memory.reserve(&mut state.objects, primitives.len())?;
    state
        .objects
        .extend((0..primitives.len()).map(|id| id as u32));
    state.allocate()?;
    state.tree[0] = [LEAF, 0, 0];
    let bounds = if let Some(first) = primitives.first() {
        let mut merged = Box3::from_bounds(*first);
        for primitive in &primitives[1..] {
            let new = Box3::from_bounds(*primitive);
            for axis in 0..3 {
                // Vector3::min/max put the NEW vector first: equal signed zeros
                // come from the most recent primitive, unlike partition extrema.
                merged.0[axis] = min_first(new.0[axis], merged.0[axis]);
                merged.0[axis + 3] = max_first(new.0[axis + 3], merged.0[axis + 3]);
            }
        }
        state.subdivide(Frame {
            start: 0,
            end: primitives.len(),
            node: 0,
            depth: 1,
            grid: merged,
            bounds: merged,
        })?;
        merged.0.map(f32::to_bits)
    } else {
        [EMPTY_BITS; 6]
    };
    let tree_words = state.tree.len().checked_mul(3).ok_or(Error::Limit)?;
    let size = tree_words
        .checked_add(primitives.len())
        .and_then(|n| n.checked_mul(4))
        .and_then(|n| n.checked_add(32))
        .ok_or(Error::Limit)?;
    if size > limits.max_bytes {
        return Err(Error::Limit);
    }
    let mut bytes = Vec::new();
    state.memory.reserve(&mut bytes, size)?;
    if bytes.capacity() > limits.max_bytes {
        return Err(Error::Limit);
    }
    for bits in bounds {
        bytes.extend_from_slice(&bits.to_le_bytes());
    }
    bytes.extend_from_slice(&(tree_words as u32).to_le_bytes());
    for node in &state.tree {
        for word in node {
            bytes.extend_from_slice(&word.to_le_bytes());
        }
    }
    bytes.extend_from_slice(&(primitives.len() as u32).to_le_bytes());
    for object in &state.objects {
        bytes.extend_from_slice(&object.to_le_bytes());
    }
    let build_work = state.work;
    let build_peak = state.memory.peak;
    drop(state);
    // Tree/indices have dropped. Validate the final bytes with output capacity
    // still charged, avoiding an unaccounted overlap with parser scratch.
    let view = View::parse(
        &bytes,
        primitives.len(),
        Limits {
            max_memory_bytes: limits
                .max_memory_bytes
                .checked_sub(bytes.capacity())
                .ok_or(Error::Limit)?,
            ..limits
        },
    )?;
    let checked = view.stats();
    let stats = Stats {
        primitive_count: primitives.len(),
        tree_words,
        node_count: checked.node_count,
        max_depth: checked.max_depth,
        build_work,
        resident_bytes: bytes.capacity(),
        peak_bytes: build_peak.max(
            bytes
                .capacity()
                .checked_add(checked.validation_peak_bytes)
                .ok_or(Error::Limit)?,
        ),
    };
    Ok(Bih { bytes, stats })
}
impl Builder<'_> {
    fn visit(&mut self) -> Result<(), Error> {
        self.work = self.work.checked_add(1).ok_or(Error::WorkLimit)?;
        if self.work > self.limits.max_work {
            return Err(Error::WorkLimit);
        }
        Ok(())
    }
    fn allocate(&mut self) -> Result<usize, Error> {
        let max_nodes = self.limits.max_tree_words / 3;
        if self.tree.len() >= max_nodes {
            return Err(Error::Limit);
        }
        if self.tree.len() == self.tree.capacity() {
            let next = self.tree.capacity().saturating_mul(2).max(1).min(max_nodes);
            self.memory.reserve(&mut self.tree, next)?;
        }
        let node = self.tree.len();
        self.tree.push([0; 3]);
        Ok(node)
    }
    fn leaf(&mut self, frame: Frame) {
        self.tree[frame.node][0] = LEAF | frame.start as u32;
        self.tree[frame.node][1] = (frame.end - frame.start) as u32;
    }
    fn subdivide(&mut self, mut frame: Frame) -> Result<(), Error> {
        self.visit()?;
        if frame.depth > self.limits.max_depth {
            return Err(Error::Limit);
        }
        if frame.end - frame.start <= self.leaf || frame.depth >= 64 {
            self.leaf(frame);
            return Ok(());
        }
        let mut previous_axis = None;
        let mut previous_split = f32::NAN;
        let mut previous_clip = f32::NAN;
        let mut was_left = true;
        let (axis, split, boundary, clip_left, clip_right) = loop {
            self.visit()?;
            let extent = [
                frame.grid.0[3] - frame.grid.0[0],
                frame.grid.0[4] - frame.grid.0[1],
                frame.grid.0[5] - frame.grid.0[2],
            ];
            if extent.iter().any(|value| *value < 0.0) {
                return Err(Error::Partition);
            }
            for axis in 0..3 {
                if frame.bounds.0[axis + 3] < frame.grid.0[axis]
                    || frame.bounds.0[axis] > frame.grid.0[axis + 3]
                {
                    return Err(Error::Partition);
                }
            }
            let axis = primary_axis(extent);
            let split = 0.5_f32 * (frame.grid.0[axis] + frame.grid.0[axis + 3]);
            let mut left_clip = f32::NEG_INFINITY;
            let mut right_clip = f32::INFINITY;
            let mut minimum = f32::INFINITY;
            let mut maximum = f32::NEG_INFINITY;
            let mut at = frame.start;
            let mut boundary = frame.end;
            while at < boundary {
                self.visit()?;
                let primitive = self.primitives[self.objects[at] as usize].bits;
                let low = f32::from_bits(primitive[axis]);
                let high = f32::from_bits(primitive[axis + 3]);
                let center = (low + high) * 0.5_f32;
                if center <= split {
                    at += 1;
                    if left_clip < high {
                        left_clip = high;
                    }
                } else {
                    boundary -= 1;
                    self.objects.swap(at, boundary);
                    if right_clip > low {
                        right_clip = low;
                    }
                }
                minimum = min_first(minimum, low);
                maximum = max_first(maximum, high);
            }
            if minimum > frame.bounds.0[axis] && maximum < frame.bounds.0[axis + 3] {
                let old_width = frame.bounds.0[axis + 3] - frame.bounds.0[axis];
                let new_width = maximum - minimum;
                if 1.3_f32 * new_width < old_width {
                    let child = self.allocate()?;
                    self.tree[frame.node] = [
                        ((axis as u32) << 30) | BVH2 | (child * 3) as u32,
                        minimum.to_bits(),
                        maximum.to_bits(),
                    ];
                    frame.bounds.0[axis] = minimum;
                    frame.bounds.0[axis + 3] = maximum;
                    frame.node = child;
                    frame.depth += 1;
                    return self.subdivide(frame);
                }
            }
            if boundary == frame.end {
                if previous_axis == Some(axis) && fuzzy_eq(previous_split, split) {
                    self.leaf(frame);
                    return Ok(());
                }
                frame.grid.0[axis + 3] = split;
                previous_clip = if left_clip <= split {
                    was_left = true;
                    left_clip
                } else {
                    f32::NAN
                };
            } else if boundary == frame.start {
                if previous_axis == Some(axis) && fuzzy_eq(previous_split, split) {
                    self.leaf(frame);
                    return Ok(());
                }
                frame.grid.0[axis] = split;
                previous_clip = if right_clip >= split {
                    was_left = false;
                    right_clip
                } else {
                    f32::NAN
                };
            } else {
                if let Some(previous_axis) = previous_axis
                    && !previous_clip.is_nan()
                {
                    let child = self.allocate()?;
                    self.tree[frame.node] = if was_left {
                        [
                            ((previous_axis as u32) << 30) | (child * 3) as u32,
                            previous_clip.to_bits(),
                            f32::INFINITY.to_bits(),
                        ]
                    } else {
                        [
                            ((previous_axis as u32) << 30) | (child * 3 - 3) as u32,
                            f32::NEG_INFINITY.to_bits(),
                            previous_clip.to_bits(),
                        ]
                    };
                    frame.node = child;
                    frame.depth += 1;
                    if frame.depth > self.limits.max_depth {
                        return Err(Error::Limit);
                    }
                }
                break (axis, split, boundary, left_clip, right_clip);
            }
            previous_axis = Some(axis);
            previous_split = split;
        };
        let left = self.allocate()?;
        let right = self.allocate()?;
        self.tree[frame.node] = [
            ((axis as u32) << 30) | (left * 3) as u32,
            clip_left.to_bits(),
            clip_right.to_bits(),
        ];
        let mut a = frame;
        a.end = boundary;
        a.node = left;
        a.depth += 1;
        a.grid.0[axis + 3] = split;
        a.bounds.0[axis + 3] = clip_left;
        let mut b = frame;
        b.start = boundary;
        b.node = right;
        b.depth += 1;
        b.grid.0[axis] = split;
        b.bounds.0[axis] = clip_right;
        self.subdivide(a)?;
        self.subdivide(b)
    }
}
fn min_first(a: f32, b: f32) -> f32 {
    if b < a { b } else { a }
}
fn max_first(a: f32, b: f32) -> f32 {
    if a < b { b } else { a }
}
fn primary_axis(extent: [f32; 3]) -> usize {
    let [x, y, z] = extent.map(f32::abs);
    if x > y {
        if x > z { 0 } else { 2 }
    } else if y > z {
        1
    } else {
        2
    }
}
fn fuzzy_eq(a: f32, b: f32) -> bool {
    let magnitude = a.abs() + 1.0_f32;
    let epsilon = if magnitude == f32::INFINITY {
        0.00001_f32
    } else {
        0.00001_f32 * magnitude
    };
    a == b || (a - b).abs() <= epsilon
}
