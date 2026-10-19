// SPDX-License-Identifier: GPL-3.0-or-later
// Authored 2026-10-11: prepare-all/commit-once static tile transactions.
// Modified 2026-10-20: reserve model tables only when capacity is exhausted.
use super::*;

struct Pending {
    ordinal: usize,
    binding: Option<Binding>,
    handle: usize,
}
struct AddedModel {
    handle: usize,
    value: Model,
}
pub(super) struct Plan {
    map_id: u32,
    new_map: Option<Map>,
    pending: Vec<Pending>,
    added: Vec<AddedModel>,
    index: Option<Vec<usize>>,
    model_table: Option<Vec<Option<Model>>>,
    map_table: Option<Vec<Map>>,
    tile_table: Option<Vec<Tile>>,
    tile: Option<Tile>,
    report: Option<LoadReport>,
}
impl Plan {
    fn empty(map_id: u32) -> Self {
        Self {
            map_id,
            new_map: None,
            pending: Vec::new(),
            added: Vec::new(),
            index: None,
            model_table: None,
            map_table: None,
            tile_table: None,
            tile: None,
            report: None,
        }
    }
    /// All allocations, overflow checks, source checks and validation precede
    /// this method. Moving prepared resources performs no allocation or I/O.
    pub fn commit(mut self, core: &mut Core) -> Option<LoadReport> {
        if let Some(mut table) = self.model_table.take() {
            table.append(&mut core.models);
            let desired = self
                .added
                .iter()
                .map(|m| m.handle + 1)
                .max()
                .unwrap_or(table.len());
            table.resize_with(desired.max(table.len()), || None);
            core.models = table;
        }
        for model in self.added {
            core.models[model.handle] = Some(model.value);
        }
        if let Some(index) = self.index {
            core.model_index = index;
        }
        if let Some(map) = self.new_map.take() {
            let mut table = self.map_table.take().expect("prepared map table");
            table.append(&mut core.maps);
            let at = table
                .binary_search_by_key(&map.id, |m| m.id)
                .expect_err("new map");
            table.insert(at, map);
            core.maps = table;
        }
        let Ok(mi) = core.map(self.map_id) else {
            return self.report;
        };
        let map = &mut core.maps[mi];
        for update in self.pending {
            let slot = &mut map.slots[update.ordinal];
            if let Some(binding) = update.binding {
                slot.binding = Some(binding);
            }
            let binding = slot.binding.as_mut().expect("prepared binding");
            if binding.references == 0 {
                binding.model = Some(update.handle);
                core.models[update.handle]
                    .as_mut()
                    .expect("prepared model")
                    .references += 1;
            }
            binding.references += 1;
        }
        if let Some(tile) = self.tile {
            if let Some(at) = map
                .tiles
                .iter()
                .position(|t| (t.x, t.y) == (tile.x, tile.y))
            {
                map.tiles[at] = tile;
            } else {
                let mut table = self.tile_table.take().expect("prepared tile table");
                table.append(&mut map.tiles);
                table.push(tile);
                map.tiles = table;
            }
        }
        self.report
    }
}

fn component(limits: Limits, memory: &Memory) -> assembly::Limits {
    assembly::Limits {
        max_spawns: limits.max_slots,
        max_associations: limits.max_references.min(assembly::MAX_ASSOCIATIONS),
        max_tile_bytes: limits.max_file_bytes.min(assembly::MAX_TILE_BYTES),
        max_output_bytes: limits.max_file_bytes,
        max_memory_bytes: memory.free().min(assembly::MAX_MEMORY_BYTES),
        bih: crate::bih::Limits {
            max_memory_bytes: memory.free().min(crate::bih::MAX_MEMORY_BYTES),
            ..Default::default()
        },
        ..Default::default()
    }
}
fn tree<S: Source>(
    source: &mut S,
    core: &Core,
    id: u32,
    limits: Limits,
    memory: &mut Memory,
    work: &mut Work,
) -> Result<Map, Error> {
    if core.maps.len() >= limits.max_maps {
        return Err(Error::Limit);
    }
    let bytes = source::read(source, Artifact::Tree { map_id: id }, limits, memory, work)?
        .ok_or(Error::MissingTree)?;
    work.charge(bytes.len())?;
    let capacity = bytes.capacity();
    let mut c = component(limits, memory);
    c.max_memory_bytes = add(memory.free(), capacity)?.min(assembly::MAX_MEMORY_BYTES);
    c.bih.max_memory_bytes = c.max_memory_bytes.min(crate::bih::MAX_MEMORY_BYTES);
    let tree = assembly::OwnedTree::parse(bytes, c)?;
    memory.release(capacity);
    memory.observe(tree.validation_peak_bytes())?;
    memory.charge(tree.resident_bytes())?;
    let count = tree.primitive_count();
    if add(core.stats().primitive_slots, count)? > limits.max_slots {
        return Err(Error::Limit);
    }
    let slots = constraints(&tree, memory, work)?;
    Ok(Map {
        id,
        tree,
        slots,
        tiles: Vec::new(),
    })
}

/// Reachable root/clip constraints for each intrinsic primitive ordinal.
/// These prove admitted placements fit their tree traversal regions, not the
/// exact unseen assembler primitive or canonical constructor bytes.
fn constraints(
    tree: &assembly::OwnedTree,
    memory: &mut Memory,
    work: &mut Work,
) -> Result<Vec<Slot>, Error> {
    let mut slots = Vec::new();
    memory.reserve(&mut slots, tree.primitive_count())?;
    let view = tree.bih();
    let root = view.bounds_bits().map(f32::from_bits);
    slots.resize_with(tree.primitive_count(), || Slot {
        bounds: root,
        binding: None,
    });
    if slots.is_empty() {
        return Ok(slots);
    }
    let mut stack = [(0usize, [0.0f32; 6]); crate::bih::MAX_DEPTH];
    let mut used = 1usize;
    stack[0] = (0, root);
    while used != 0 {
        used -= 1;
        let (node, bound) = stack[used];
        work.charge(1)?;
        let head = view.query_word(node).ok_or(Error::Association)?;
        let a = view.query_word(node + 1).ok_or(Error::Association)?;
        let b = view.query_word(node + 2).ok_or(Error::Association)?;
        let axis = (head >> 30) as usize;
        let offset = (head & 0x1fff_ffff) as usize;
        if axis == 3 {
            work.charge(a as usize)?;
            for at in offset..add(offset, a as usize)? {
                let ordinal = view.query_object(at).ok_or(Error::Association)? as usize;
                slots.get_mut(ordinal).ok_or(Error::Association)?.bounds = bound;
            }
        } else {
            let lo = f32::from_bits(a);
            let hi = f32::from_bits(b);
            let mut push = |node, bound| -> Result<(), Error> {
                *stack.get_mut(used).ok_or(Error::Association)? = (node, bound);
                used += 1;
                Ok(())
            };
            if head & 0x2000_0000 != 0 {
                let mut next = bound;
                next[axis] = next[axis].max(lo);
                next[axis + 3] = next[axis + 3].min(hi);
                push(offset, next)?;
            } else {
                if hi != f32::INFINITY {
                    let mut next = bound;
                    next[axis] = next[axis].max(hi);
                    push(add(offset, 3)?, next)?;
                }
                if lo != f32::NEG_INFINITY {
                    let mut next = bound;
                    next[axis + 3] = next[axis + 3].min(lo);
                    push(offset, next)?;
                }
            }
        }
    }
    Ok(slots)
}
fn name(bytes: &[u8]) -> Result<(), Error> {
    if bytes.is_empty()
        || bytes.len() > placement::MAX_NAME_BYTES
        || bytes == b"."
        || bytes == b".."
        || bytes
            .iter()
            .any(|b| !(b' '..=b'~').contains(b) || matches!(*b, b'/' | b'\\' | b':'))
    {
        Err(Error::Name)
    } else {
        Ok(())
    }
}
fn equivalent(a: &[u8], b: &[u8]) -> bool {
    a.len() == b.len() && a[0] & !2 == b[0] & !2 && a[1..] == b[1..]
}
fn append_capacity<T>(v: &mut Vec<T>, memory: &mut Memory, ceiling: usize) -> Result<(), Error> {
    let needed = add(v.len(), 1)?;
    if needed > ceiling {
        return Err(Error::Limit);
    }
    if needed <= v.capacity() {
        return Ok(());
    }
    memory.reserve(v, needed.max(v.capacity().saturating_mul(2)).min(ceiling))
}

struct Models<'a> {
    core: &'a Core,
    added: Vec<AddedModel>,
    names: Vec<usize>, // name order, indexes into added
    free_cursor: usize,
    appended: usize,
}
impl<'a> Models<'a> {
    fn new(core: &'a Core) -> Self {
        Self {
            core,
            added: Vec::new(),
            names: Vec::new(),
            free_cursor: 0,
            appended: 0,
        }
    }
    fn obtain<S: Source>(
        &mut self,
        name: &[u8],
        source: &mut S,
        limits: Limits,
        memory: &mut Memory,
        work: &mut Work,
    ) -> Result<usize, Error> {
        work.charge(1)?;
        if let Some(handle) = self.core.model(name) {
            return Ok(handle);
        }
        let pos = match self
            .names
            .binary_search_by(|n| self.added[*n].value.name.as_slice().cmp(name))
        {
            Ok(at) => return Ok(self.added[self.names[at]].handle),
            Err(at) => at,
        };
        if add(self.core.model_index.len(), self.added.len())? >= limits.max_models {
            return Err(Error::Limit);
        }
        let bytes = source::read(source, Artifact::Model { name }, limits, memory, work)?
            .ok_or(Error::MissingModel)?;
        work.charge(bytes.len())?;
        let cap = bytes.capacity();
        let available = add(memory.free(), cap)?.min(vmo::MAX_MEMORY_BYTES);
        let model = vmo::Owned::parse(
            bytes,
            vmo::Limits {
                max_bytes: limits.max_file_bytes,
                max_memory_bytes: available,
                bih: crate::bih::Limits {
                    max_memory_bytes: available.min(crate::bih::MAX_MEMORY_BYTES),
                    ..Default::default()
                },
                ..Default::default()
            },
        )?;
        memory.release(cap);
        memory.observe(model.validation_peak_bytes())?;
        memory.charge(model.resident_bytes())?;
        let owned_name = memory.copy(name)?;
        while self.free_cursor < self.core.models.len()
            && self.core.models[self.free_cursor].is_some()
        {
            work.charge(1)?;
            self.free_cursor += 1;
        }
        let handle = if self.free_cursor < self.core.models.len() {
            let handle = self.free_cursor;
            self.free_cursor += 1;
            handle
        } else {
            let handle = add(self.core.models.len(), self.appended)?;
            self.appended += 1;
            handle
        };
        append_capacity(&mut self.added, memory, limits.max_models)?;
        append_capacity(&mut self.names, memory, limits.max_models)?;
        work.charge(self.names.len() - pos)?;
        self.names.insert(pos, self.added.len());
        self.added.push(AddedModel {
            handle,
            value: Model {
                name: owned_name,
                model,
                references: 0,
            },
        });
        Ok(handle)
    }
}

#[allow(clippy::too_many_arguments)]
pub(super) fn prepare<S: Source>(
    source: &mut S,
    core: &Core,
    id: u32,
    coordinate: Option<(u16, u8)>,
    optional: bool,
    options: Options,
    limits: Limits,
    memory: &mut Memory,
    work: &mut Work,
) -> Result<Plan, Error> {
    let mut plan = Plan::empty(id);
    if core.map(id).is_err() {
        plan.new_map = Some(tree(source, core, id, limits, memory, work)?);
        let mut table = Vec::new();
        memory.reserve(&mut table, add(core.maps.len(), 1)?)?;
        plan.map_table = Some(table);
    }
    let map = match &plan.new_map {
        Some(map) => map,
        None => &core.maps[core.map(id).expect("known map")],
    };
    let Some((x, y)) = coordinate else {
        return Ok(plan);
    };
    if map.slots.is_empty() {
        return Err(Error::EmptyTree);
    }
    let prior = map.tiles.iter().find(|t| (t.x, t.y) == (x, y));
    if prior.is_none() && core.stats().tiles >= limits.max_tiles {
        return Err(Error::Limit);
    }
    let own = source::read(
        source,
        Artifact::Tile { map_id: id, x, y },
        limits,
        memory,
        work,
    )?;
    let indices = source::read(
        source,
        Artifact::Index { map_id: id, x, y },
        limits,
        memory,
        work,
    )?;
    if optional && own.is_none() && indices.is_none() {
        return Ok(plan);
    }
    let indices = indices.ok_or(Error::MissingIndex)?;
    let mut payload = own;
    let mut used_map = id;
    while payload.is_none() {
        work.charge(1)?;
        used_map = core.parent(used_map)?.ok_or(Error::MissingTile)?;
        payload = source::read(
            source,
            Artifact::Tile {
                map_id: used_map,
                x,
                y,
            },
            limits,
            memory,
            work,
        )?;
    }
    let payload = payload.expect("present payload");
    work.charge(add(payload.len(), indices.len())?)?;
    let c = component(limits, memory);
    let tile = assembly::TileView::parse(&payload, c)?;
    let index = assembly::IndexView::parse(&indices, map.slots.len(), c)?;
    memory.observe(index.validation_peak_bytes())?;
    if tile.count() != index.count() {
        return Err(Error::Association);
    }
    let mut models = Models::new(core);
    let mut references = Vec::new();
    let old_count = prior.map_or(0, |t| t.references.len());
    // Conservatively reserve all rows, even path-only omissions, before iteration.
    memory.reserve(&mut references, add(old_count, tile.count())?)?;
    if let Some(old) = prior {
        references.extend_from_slice(&old.references);
    }
    memory.reserve(&mut plan.pending, tile.count())?;
    let mut cursor = 12usize;
    let mut added_refs = 0usize;
    for (record, ordinal) in tile.records().zip(index.indices()) {
        work.charge(1)?;
        let end = add(cursor, add(62, record.name().len())?)?;
        let wire = payload.get(cursor..end).ok_or(Error::Association)?;
        cursor = end;
        let slot = &map.slots[ordinal as usize];
        let bounds = record
            .bounds_bits()
            .ok_or(Error::Association)?
            .map(f32::from_bits);
        for axis in 0..3 {
            if bounds[axis] < slot.bounds[axis] || bounds[axis + 3] > slot.bounds[axis + 3] {
                return Err(Error::Association);
            }
        }
        if record.flags() & 4 != 0 && !options.load_path_only_models {
            continue;
        }
        name(record.name())?;
        if let Some(old) = &slot.binding
            && !equivalent(&old.record, wire)
        {
            return Err(Error::Association);
        }
        let instance = if slot.binding.is_none() {
            Some(query::model::Instance::new(&record)?)
        } else {
            None
        };
        let handle = if let Some(h) = slot.binding.as_ref().and_then(|b| b.model) {
            h
        } else {
            models.obtain(record.name(), source, limits, memory, work)?
        };
        let binding = if let Some(instance) = instance {
            Some(Binding {
                record: memory.copy(wire)?,
                instance,
                model: None,
                references: 0,
            })
        } else {
            None
        };
        if slot
            .binding
            .as_ref()
            .is_some_and(|b| b.references == usize::MAX)
        {
            return Err(Error::Limit);
        }
        plan.pending.push(Pending {
            ordinal: ordinal as usize,
            binding,
            handle,
        });
        references.push(ordinal);
        added_refs += 1;
    }
    if add(core.stats().tile_references, added_refs)? > limits.max_references {
        return Err(Error::Limit);
    }
    // Partial intrinsic correspondence: every known binding preserves sorted-ID
    // ordinal order. Unloaded-but-known bindings remain until the map is dropped.
    work.charge(map.slots.len())?;
    plan.pending.sort_unstable_by_key(|p| p.ordinal);
    let mut updates = plan.pending.iter().peekable();
    let mut last = None;
    for (ordinal, slot) in map.slots.iter().enumerate() {
        let update = if updates.peek().is_some_and(|p| p.ordinal == ordinal) {
            updates.next()
        } else {
            None
        };
        let binding = update
            .and_then(|p| p.binding.as_ref())
            .or(slot.binding.as_ref());
        if let Some(binding) = binding {
            if last.is_some_and(|id| id >= binding.id()) {
                return Err(Error::Association);
            }
            last = Some(binding.id());
        }
    }
    let records = tile.count();
    memory.release(payload.capacity());
    drop(payload);
    memory.release(indices.capacity());
    drop(indices);
    let loads = prior.map_or(Ok(1), |t| t.loads.checked_add(1).ok_or(Error::Limit))?;
    if prior.is_none() {
        let mut table = Vec::new();
        memory.reserve(&mut table, add(map.tiles.len(), 1)?)?;
        plan.tile_table = Some(table);
    }
    plan.tile = Some(Tile {
        x,
        y,
        references,
        loads,
    });
    plan.report = Some(LoadReport {
        used_map_id: used_map,
        records,
        references_added: added_refs,
        new_models: models.added.len(),
        repeated: prior.is_some(),
    });
    if !models.added.is_empty() {
        let mut index = Vec::new();
        memory.reserve(&mut index, add(core.model_index.len(), models.added.len())?)?;
        let mut old = core.model_index.iter().peekable();
        let mut new = models.names.iter().peekable();
        while let (Some(a), Some(b)) = (old.peek(), new.peek()) {
            work.charge(1)?;
            if core.models[**a].as_ref().expect("old model").name < models.added[**b].value.name {
                index.push(*old.next().expect("old"));
            } else {
                index.push(models.added[*new.next().expect("new")].handle);
            }
        }
        index.extend(old.copied());
        index.extend(new.map(|n| models.added[*n].handle));
        plan.index = Some(index);
        if models.appended != 0 {
            let mut table = Vec::new();
            memory.reserve(&mut table, add(core.models.len(), models.appended)?)?;
            plan.model_table = Some(table);
        }
    }
    // Each acquired model reference is one active slot. This bound makes the
    // commit's increments infallible without hidden post-commit validation.
    if add(
        core.stats().active_instances,
        plan.pending
            .iter()
            .filter(|p| {
                map.slots[p.ordinal]
                    .binding
                    .as_ref()
                    .is_none_or(|b| b.references == 0)
            })
            .count(),
    )? > limits.max_slots
    {
        return Err(Error::Limit);
    }
    plan.added = models.added;
    Ok(plan)
}

#[cfg(test)]
mod append_capacity_tests {
    use super::*;
    use std::mem::size_of;

    #[test]
    fn spare_capacity_does_not_reserve_or_charge_again() {
        let mut values = Vec::<usize>::with_capacity(8);
        values.push(1);
        let capacity = values.capacity();
        assert!(capacity > values.len());
        let accounted = capacity * size_of::<usize>();
        let mut memory = Memory::new(accounted, accounted);

        append_capacity(&mut values, &mut memory, capacity.saturating_mul(2)).unwrap();

        assert_eq!(values.capacity(), capacity);
        assert_eq!(memory.live, accounted);
        assert_eq!(memory.peak, accounted);
    }

    #[test]
    fn zero_item_ceiling_rejects_empty_vec_before_reservation() {
        let mut values = Vec::<usize>::with_capacity(8);
        let capacity = values.capacity();
        assert_eq!(values.len(), 0);
        let accounted = capacity * size_of::<usize>();
        let mut memory = Memory::new(accounted, accounted);

        assert_eq!(
            append_capacity(&mut values, &mut memory, 0),
            Err(Error::Limit)
        );
        assert_eq!(values.len(), 0);
        assert_eq!(values.capacity(), capacity);
        assert_eq!(memory.live, accounted);
        assert_eq!(memory.peak, accounted);
    }

    #[test]
    fn lowered_item_ceiling_rejects_before_reservation() {
        let mut values = Vec::<usize>::with_capacity(8);
        values.push(1);
        let capacity = values.capacity();
        let accounted = capacity * size_of::<usize>();
        let mut memory = Memory::new(accounted, accounted);

        let lowered_ceiling = values.len();
        assert_eq!(
            append_capacity(&mut values, &mut memory, lowered_ceiling),
            Err(Error::Limit)
        );
        assert_eq!(values.len(), 1);
        assert_eq!(values.capacity(), capacity);
        assert_eq!(memory.live, accounted);
        assert_eq!(memory.peak, accounted);
    }

    #[test]
    fn growth_obeys_exact_memory_boundary_and_one_byte_short_fails() {
        fn initial() -> Vec<usize> {
            vec![1]
        }

        let mut exact_values = initial();
        let old_capacity = exact_values.capacity();
        let requested = (exact_values.len() + 1).max(old_capacity.saturating_mul(2));
        let item_ceiling = requested;
        let old_bytes = old_capacity * size_of::<usize>();
        let requested_bytes = requested * size_of::<usize>();
        let exact_limit = old_bytes + requested_bytes;
        let mut exact_memory = Memory::new(old_bytes, exact_limit);

        append_capacity(&mut exact_values, &mut exact_memory, item_ceiling).unwrap();

        assert!(exact_values.capacity() >= requested);
        assert_eq!(
            exact_memory.live,
            old_bytes + (exact_values.capacity() - old_capacity) * size_of::<usize>()
        );
        assert_eq!(exact_memory.peak, exact_limit);

        let mut short_values = initial();
        assert_eq!(short_values.capacity(), old_capacity);
        let mut short_memory = Memory::new(old_bytes, exact_limit - 1);
        assert_eq!(
            append_capacity(&mut short_values, &mut short_memory, item_ceiling),
            Err(Error::Limit)
        );
        assert_eq!(short_values.len(), 1);
        assert_eq!(short_values.capacity(), old_capacity);
        assert_eq!(short_memory.live, old_bytes);
        assert_eq!(short_memory.peak, old_bytes);
    }
}
