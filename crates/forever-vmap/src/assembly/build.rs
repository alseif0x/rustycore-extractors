// SPDX-License-Identifier: GPL-3.0-or-later
// Authored 2026-10-11: original bounded map assembly, no I/O.
use super::*;
struct Plan {
    id: u32,
    original: usize,
    start: usize,
    len: usize,
    parent: bool,
    rect: Rect,
    bound: bih::Bounds,
}
#[derive(Clone, Copy)]
struct Association {
    key: u32,
    ordinal: u32,
}

pub fn build(
    map_id: u32,
    records: &[placement::Placement<'_>],
    limits: Limits,
) -> Result<AssembledMap, Error> {
    limits.validate()?;
    if records.len() > limits.max_spawns {
        return Err(Error::Limit);
    }
    let mut input_len = 0;
    for p in records {
        if p.bounds_bits().is_none() {
            return Err(Error::MissingBounds);
        }
        input_len = add(input_len, encoded_len(p)?)?;
        if input_len > limits.max_input_bytes {
            return Err(Error::Limit);
        }
    }
    let mut memory = Memory::new(limits.max_memory_bytes);
    memory.observe(input_len)?;
    let input = placement::encode(
        records,
        limits.codec(limits.max_input_bytes.min(limits.max_memory_bytes)),
    )?;
    memory.adopt(&input)?;
    let mut plans = Vec::new();
    memory.reserve(&mut plans, records.len())?;
    let mut start = 0;
    for (original, p) in records.iter().enumerate() {
        let len = encoded_len(p)?;
        plans.push(Plan {
            id: p.id(),
            original,
            start,
            len,
            parent: p.flags() & 2 != 0,
            rect: rectangle(p)?,
            bound: bih::Bounds::from_bits(p.bounds_bits().ok_or(Error::MissingBounds)?)?,
        });
        start = add(start, len)?;
    }
    // Explicit ordinal tie-break gives native first-record-wins without a
    // stable-sort allocation, map/set allocator or dependence on input IDs.
    plans.sort_unstable_by_key(|p| (p.id, p.original));
    plans.dedup_by_key(|p| p.id);
    let count = plans.len();
    let mut associations = 0;
    let mut proof_len = 0;
    for p in &plans {
        let cells = p.rect.count()?;
        if cells > limits.max_tiles {
            return Err(Error::Limit);
        }
        associations = add(associations, cells)?;
        if associations > limits.max_associations {
            return Err(Error::Limit);
        }
        proof_len = add(proof_len, p.len)?;
    }
    let mut ordered = Vec::new();
    memory.reserve(&mut ordered, proof_len)?;
    if ordered.capacity() > limits.max_input_bytes {
        return Err(Error::Limit);
    }
    for p in &mut plans {
        let bytes = input
            .get(p.start..add(p.start, p.len)?)
            .ok_or(Error::Framing)?;
        p.start = ordered.len();
        ordered.extend_from_slice(bytes);
    }
    memory.release(input);

    let mut bounds = Vec::new();
    memory.reserve(&mut bounds, count)?;
    bounds.extend(plans.iter().map(|p| p.bound));
    let tree = bih::build(&bounds, 3, limits.tree(memory.live)?)?;
    let tree_stats = tree.stats();
    memory.observe(tree_stats.peak_bytes)?;
    memory.live = add(memory.live, tree_stats.resident_bytes)?;
    memory.release(bounds);
    let tree_size = add(12, tree.bytes().len())?;
    if tree_size > limits.max_output_bytes {
        return Err(Error::Limit);
    }
    let mut map_tree = Vec::new();
    memory.reserve(&mut map_tree, tree_size)?;
    map_tree.extend_from_slice(MAGIC);
    map_tree.extend_from_slice(b"NODE");
    map_tree.extend_from_slice(tree.bytes());
    memory.release(tree.into_bytes());

    // Cardinality is checked before the first range expansion or its reserve.
    let mut refs = Vec::new();
    memory.reserve(&mut refs, associations)?;
    for (ordinal, p) in plans.iter().enumerate() {
        for x in p.rect.low_x..=p.rect.high_x {
            for y in p.rect.low_y..=p.rect.high_y {
                refs.push(Association {
                    key: (u32::from(x) << 16) | u32::from(y),
                    ordinal: ordinal as u32,
                });
            }
        }
    }
    refs.sort_unstable_by_key(|r| (r.key, plans[r.ordinal as usize].parent, r.ordinal));
    let mut tiles_count = 0;
    let mut payload_tiles = 0;
    let mut output_bytes = tree_size;
    let mut at = 0;
    while at < refs.len() {
        let end = group_end(&refs, at);
        let (index_len, payload_len) = sizes(&refs[at..end], &plans, limits)?;
        tiles_count += 1;
        payload_tiles += usize::from(payload_len.is_some());
        if tiles_count > limits.max_tiles {
            return Err(Error::Limit);
        }
        output_bytes = add(output_bytes, add(index_len, payload_len.unwrap_or(0))?)?;
        if output_bytes > limits.max_output_bytes {
            return Err(Error::Limit);
        }
        at = end;
    }
    let mut tiles = Vec::new();
    memory.reserve(&mut tiles, tiles_count)?;
    at = 0;
    while at < refs.len() {
        let end = group_end(&refs, at);
        let rows = &refs[at..end];
        let (index_len, payload_len) = sizes(rows, &plans, limits)?;
        let mut index = Vec::new();
        memory.reserve(&mut index, index_len)?;
        if index.capacity() > limits.max_tile_bytes {
            return Err(Error::Limit);
        }
        index.extend_from_slice(MAGIC);
        push(&mut index, rows.len() as u32);
        for r in rows {
            push(&mut index, r.ordinal);
        }
        let payload = if let Some(size) = payload_len {
            let mut bytes = Vec::new();
            memory.reserve(&mut bytes, size)?;
            if bytes.capacity() > limits.max_tile_bytes {
                return Err(Error::Limit);
            }
            bytes.extend_from_slice(MAGIC);
            push(&mut bytes, rows.len() as u32);
            for r in rows {
                let p = &plans[r.ordinal as usize];
                bytes.extend_from_slice(
                    ordered
                        .get(p.start..add(p.start, p.len)?)
                        .ok_or(Error::Framing)?,
                );
            }
            Some(bytes)
        } else {
            None
        };
        tiles.push(Tile {
            x: (rows[0].key >> 16) as u16,
            y: (rows[0].key & 0xff) as u16,
            payload,
            index,
        });
        at = end;
    }
    memory.release(refs);
    let validation_limits = Limits {
        max_memory_bytes: limits
            .max_memory_bytes
            .checked_sub(memory.live)
            .ok_or(Error::Limit)?,
        ..limits
    };
    let view = Tree::parse(&map_tree, count, validation_limits)?;
    memory.observe(view.bih().stats().validation_peak_bytes)?;
    for tile in &tiles {
        let index = IndexView::parse(&tile.index, count, validation_limits)?;
        memory.observe(index.validation_peak_bytes())?;
        if let Some(payload) = &tile.payload {
            let view = TileView::parse(payload, validation_limits)?;
            if view.count() != index.count() {
                return Err(Error::Association);
            }
            let mut at = 12;
            for (record, ordinal) in view.records().zip(index.indices()) {
                let p = plans.get(ordinal as usize).ok_or(Error::Index)?;
                let end = add(at, encoded_len(&record)?)?;
                if payload.get(at..end) != ordered.get(p.start..add(p.start, p.len)?) {
                    return Err(Error::Association);
                }
                at = end;
            }
        } else {
            let mut previous = None;
            for ordinal in index.indices() {
                if previous.is_some_and(|old| old >= ordinal) || !plans[ordinal as usize].parent {
                    return Err(Error::Association);
                }
                previous = Some(ordinal);
            }
        }
    }
    memory.release(plans);
    let stats = Stats {
        input_records: records.len(),
        unique_spawns: count,
        duplicates: records.len() - count,
        tiles: tiles_count,
        associations,
        payload_tiles,
        parent_only_tiles: tiles_count - payload_tiles,
        output_bytes,
        proof_bytes: ordered.len(),
        resident_bytes: memory.live,
        peak_bytes: memory.peak,
        bih_nodes: tree_stats.node_count,
        bih_depth: tree_stats.max_depth,
        bih_work: tree_stats.build_work,
    };
    Ok(AssembledMap {
        map_id,
        tree: map_tree,
        ordered,
        tiles,
        stats,
    })
}
fn group_end(refs: &[Association], at: usize) -> usize {
    let key = refs[at].key;
    at + refs[at..].partition_point(|r| r.key == key)
}
fn sizes(
    rows: &[Association],
    plans: &[Plan],
    limits: Limits,
) -> Result<(usize, Option<usize>), Error> {
    let index = add(12, mul(rows.len(), 4)?)?;
    let payload = if plans[rows[0].ordinal as usize].parent {
        None
    } else {
        let mut size = 12;
        for r in rows {
            size = add(size, plans[r.ordinal as usize].len)?;
        }
        Some(size)
    };
    if index > limits.max_tile_bytes || payload.is_some_and(|n| n > limits.max_tile_bytes) {
        return Err(Error::Limit);
    }
    Ok((index, payload))
}
