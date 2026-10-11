// SPDX-License-Identifier: GPL-3.0-or-later
// Authored 2026-10-11: strict complete VMO reader and geometric BIH checks.
use super::*;

pub(super) fn parse(bytes: &[u8], limits: Limits) -> Result<View<'_>, Error> {
    limits.validate()?;
    if bytes.len() > limits.max_bytes {
        return Err(Error::Limit);
    }
    let mut c = Cursor { bytes, at: 0 };
    if c.take(8)? != MAGIC || c.take(4)? != b"WMOD" || c.word()? != 8 {
        return Err(Error::Framing);
    }
    let flags = c.word()?;
    let root = c.word()?;
    if flags > 1 {
        return Err(Error::Flags);
    }
    let mut memory = Memory::new(limits.max_memory_bytes);
    let mut groups = Vec::new();
    let mut stats = Stats {
        bytes: bytes.len(),
        ..Stats::default()
    };
    if c.at == bytes.len() {
        return Ok(View {
            bytes,
            flags,
            root,
            groups,
            group_bih: None,
            stats,
        });
    }
    if c.take(4)? != b"GMOD" {
        return Err(Error::Framing);
    }
    let count = c.word()? as usize;
    if count == 0 {
        return Err(Error::Framing);
    }
    if count > limits.max_groups {
        return Err(Error::Limit);
    }
    // At least 44 bytes per group plus GBIH and its minimum 44-byte tree.
    // Reject impossible counts before reserving descriptors.
    if add(mul(count, 44)?, 48)? > bytes.len() - c.at {
        return Err(Error::Framing);
    }
    memory.reserve(&mut groups, count)?;
    for _ in 0..count {
        let group = read_group(&mut c, limits, &mut memory)?;
        stats.vertices = add(stats.vertices, group.vertex_count())?;
        stats.triangles = add(stats.triangles, group.triangle_count())?;
        stats.liquid_groups = add(stats.liquid_groups, usize::from(group.liquid.is_some()))?;
        groups.push(group);
    }
    let group_bih = read_tree(&mut c, b"GBIH", count, limits, &mut memory)?;
    if c.at != bytes.len() {
        return Err(Error::Framing);
    }
    containment(&group_bih, |index| {
        bounds(groups.get(index).ok_or(Error::Index)?.bounds)
    })?;
    stats.groups = count;
    stats.resident_bytes = memory.live;
    stats.peak_bytes = memory.peak;
    Ok(View {
        bytes,
        flags,
        root,
        groups,
        group_bih: Some(group_bih),
        stats,
    })
}

fn read_group<'a>(
    c: &mut Cursor<'a>,
    limits: Limits,
    memory: &mut Memory,
) -> Result<Group<'a>, Error> {
    let mut bound = [0; 6];
    for value in &mut bound {
        *value = c.word()?;
    }
    bounds(bound)?;
    let flags = c.word()?;
    let id = c.word()?;
    let vertices = c.array(b"VERT", limits.max_vertices)?;
    for bits in vertices.as_chunks::<4>().0 {
        finite(u32::from_le_bytes(*bits))?;
    }
    if vertices.is_empty() {
        return Ok(Group {
            bounds: bound,
            flags,
            id,
            vertices,
            triangles: &[],
            mesh_bih: None,
            liquid: None,
        });
    }
    let triangles = c.array(b"TRIM", limits.max_triangles)?;
    for index in triangles.as_chunks::<4>().0 {
        if u32::from_le_bytes(*index) as usize >= vertices.len() / 12 {
            return Err(Error::Index);
        }
    }
    let mesh_bih = read_tree(c, b"MBIH", triangles.len() / 12, limits, memory)?;
    containment(&mesh_bih, |index| {
        let tri = triple(triangles, index)?;
        triangle_bounds([
            triple(vertices, tri[0] as usize)?,
            triple(vertices, tri[1] as usize)?,
            triple(vertices, tri[2] as usize)?,
        ])
    })?;
    if c.take(4)? != b"LIQU" {
        return Err(Error::Framing);
    }
    let size = c.word()? as usize;
    let liquid = if size == 0 {
        None
    } else {
        Some(read_liquid(c.take(size)?, limits)?)
    };
    Ok(Group {
        bounds: bound,
        flags,
        id,
        vertices,
        triangles,
        mesh_bih: Some(mesh_bih),
        liquid,
    })
}
fn read_liquid(bytes: &[u8], limits: Limits) -> Result<Liquid<'_>, Error> {
    let mut c = Cursor { bytes, at: 0 };
    let dimensions = [c.word()?, c.word()?];
    let corner = [c.word()?, c.word()?, c.word()?];
    for value in corner {
        finite(value)?;
    }
    let type_id = c.word()?;
    let [x, y] = dimensions.map(|v| v as usize);
    // Safe converter output has a grid or 0/0. The native copy constructor can
    // overread unilateral-zero surfaces, which are outside this profile.
    let (heights, tiles) = if x == 0 && y == 0 {
        (1, 0)
    } else if x != 0 && y != 0 {
        (mul(add(x, 1)?, add(y, 1)?)?, mul(x, y)?)
    } else {
        return Err(Error::Liquid);
    };
    if heights > limits.max_liquid_heights || tiles > limits.max_liquid_tiles {
        return Err(Error::Limit);
    }
    let heights = c.take(mul(heights, 4)?)?;
    for bits in heights.as_chunks::<4>().0 {
        finite(u32::from_le_bytes(*bits))?;
    }
    let tiles = c.take(tiles)?;
    if c.at != bytes.len() {
        return Err(Error::Liquid);
    }
    Ok(Liquid {
        dimensions,
        corner,
        type_id,
        heights,
        tiles,
    })
}
fn read_tree<'a>(
    c: &mut Cursor<'a>,
    tag: &[u8; 4],
    count: usize,
    limits: Limits,
    memory: &mut Memory,
) -> Result<bih::View<'a>, Error> {
    if c.take(4)? != tag {
        return Err(Error::Framing);
    }
    let tail = c.bytes.get(c.at..).ok_or(Error::Framing)?;
    let words = word(tail, 24)? as usize;
    if words > limits.bih.max_tree_words {
        return Err(Error::Limit);
    }
    let object_count_at = add(28, mul(words, 4)?)?;
    let objects = word(tail, object_count_at)? as usize;
    if objects > limits.bih.max_primitives {
        return Err(Error::Limit);
    }
    let size = add(add(object_count_at, 4)?, mul(objects, 4)?)?;
    let view = bih::View::parse(c.take(size)?, count, limits.tree(memory.live, 0)?)?;
    memory.observe(view.stats().validation_peak_bytes)?;
    Ok(view)
}

struct Cursor<'a> {
    bytes: &'a [u8],
    at: usize,
}
impl<'a> Cursor<'a> {
    fn take(&mut self, n: usize) -> Result<&'a [u8], Error> {
        let end = add(self.at, n)?;
        let bytes = self.bytes.get(self.at..end).ok_or(Error::Framing)?;
        self.at = end;
        Ok(bytes)
    }
    fn word(&mut self) -> Result<u32, Error> {
        word(self.take(4)?, 0)
    }
    fn array(&mut self, tag: &[u8; 4], cap: usize) -> Result<&'a [u8], Error> {
        if self.take(4)? != tag {
            return Err(Error::Framing);
        }
        let size = self.word()? as usize;
        let count = self.word()? as usize;
        if count > cap {
            return Err(Error::Limit);
        }
        if size != add(4, mul(count, 12)?)? {
            return Err(Error::Framing);
        }
        self.take(mul(count, 12)?)
    }
}

/// O(nodes + primitives), bounded-depth postorder traversal of an already
/// structurally validated tree. No extra heap or canonical rebuilding occurs.
/// Numeric containment intentionally treats equal signed zeros as equal.
fn containment(
    tree: &bih::View<'_>,
    primitive: impl Fn(usize) -> Result<bih::Bounds, Error>,
) -> Result<(), Error> {
    if tree.stats().primitive_count == 0 {
        return Ok(());
    }
    let structure = tree.bytes();
    let object_start = add(32, mul(tree.stats().tree_words, 4)?)?;
    let actual = node_bounds(structure, object_start, 0, &primitive)?;
    let outer = tree.bounds_bits().map(f32::from_bits);
    for axis in 0..3 {
        if actual[axis] < outer[axis] || actual[axis + 3] > outer[axis + 3] {
            return Err(Error::Containment);
        }
    }
    Ok(())
}
fn node_bounds(
    bytes: &[u8],
    object_start: usize,
    node: usize,
    primitive: &impl Fn(usize) -> Result<bih::Bounds, Error>,
) -> Result<[f32; 6], Error> {
    let at = add(28, mul(node, 4)?)?;
    let header = word(bytes, at)?;
    let a = word(bytes, at + 4)?;
    let b = word(bytes, at + 8)?;
    let axis = (header >> 30) as usize;
    let offset = (header & 0x1fff_ffff) as usize;
    if axis == 3 {
        let mut result = None;
        for slot in offset..add(offset, a as usize)? {
            let id = word(bytes, add(object_start, mul(slot, 4)?)?)? as usize;
            let next = primitive(id)?.bits().map(f32::from_bits);
            result = Some(match result {
                None => next,
                Some(old) => merge(old, next),
            });
        }
        return result.ok_or(Error::Containment);
    }
    let low = f32::from_bits(a);
    let high = f32::from_bits(b);
    if header & 0x2000_0000 != 0 {
        let child = node_bounds(bytes, object_start, offset, primitive)?;
        if child[axis] < low || child[axis + 3] > high {
            return Err(Error::Containment);
        }
        return Ok(child);
    }
    let left = if low == f32::NEG_INFINITY {
        None
    } else {
        let child = node_bounds(bytes, object_start, offset, primitive)?;
        if child[axis + 3] > low {
            return Err(Error::Containment);
        }
        Some(child)
    };
    let right = if high == f32::INFINITY {
        None
    } else {
        let child = node_bounds(bytes, object_start, add(offset, 3)?, primitive)?;
        if child[axis] < high {
            return Err(Error::Containment);
        }
        Some(child)
    };
    match (left, right) {
        (Some(a), Some(b)) => Ok(merge(a, b)),
        (Some(a), None) | (None, Some(a)) => Ok(a),
        _ => Err(Error::Containment),
    }
}
fn merge(mut a: [f32; 6], b: [f32; 6]) -> [f32; 6] {
    for axis in 0..3 {
        if b[axis] < a[axis] {
            a[axis] = b[axis];
        }
        if b[axis + 3] > a[axis + 3] {
            a[axis + 3] = b[axis + 3];
        }
    }
    a
}
