// SPDX-License-Identifier: GPL-3.0-or-later
// Modified 2026-10-12: shared original world preparation with typed ledgers.
//! Identical closed readers, source order, copy replay and capacity admission
//! for the native resident and private spooled world owners.
use super::{
    Error, Limits, Lookups, MAP_FILE_DATA_ID, MATERIAL_FILE_DATA_ID, MAX_LOOKUP_ROWS, Prepared,
    RecordError, TEXT_ALLOWANCE, TYPE_FILE_DATA_ID, TableFacts, Topology, TopologyEntry, add,
    capacity, plan, source, word,
};
use crate::collision::{Extractor, SourceFacts};
use forever_vmap::wmo::liquid::LiquidLookup;
use std::mem::size_of;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Failure<E> {
    World(Error),
    Collector(E),
}
impl<E> From<Error> for Failure<E> {
    fn from(e: Error) -> Self {
        Self::World(e)
    }
}
pub(super) trait Access {
    type Error: Copy;
    fn read_source(&mut self, id: u32, cap: usize) -> Result<(Vec<u8>, SourceFacts), Self::Error>;
    fn reserve_external(&mut self, bytes: usize) -> Result<(), Self::Error>;
    fn release_external(&mut self, bytes: usize) -> Result<(), Self::Error>;
    fn release_source(&mut self, bytes: usize) -> Result<(), Self::Error>;
}
impl<L: LiquidLookup + ?Sized> Access for Extractor<'_, '_, L> {
    type Error = crate::collision::Error;
    fn read_source(&mut self, id: u32, cap: usize) -> Result<(Vec<u8>, SourceFacts), Self::Error> {
        Extractor::read_source(self, id, cap)
    }
    fn reserve_external(&mut self, n: usize) -> Result<(), Self::Error> {
        Extractor::reserve_external(self, n)
    }
    fn release_external(&mut self, n: usize) -> Result<(), Self::Error> {
        Extractor::release_external(self, n)
    }
    fn release_source(&mut self, n: usize) -> Result<(), Self::Error> {
        Extractor::release_source(self, n)
    }
}
pub(super) fn topology<A: Access>(
    maps: &plan::Maps<'_>,
    collector: &mut A,
    max_bytes: usize,
) -> Result<Topology, Failure<A::Error>> {
    let count = maps.all().len();
    let requested = count
        .checked_mul(size_of::<TopologyEntry>() + size_of::<usize>())
        .ok_or(Error::Limit)?;
    if requested > max_bytes {
        return Err(Error::Limit.into());
    }
    collector
        .reserve_external(requested)
        .map_err(Failure::Collector)?;
    let mut rows = Vec::new();
    let mut by_id = Vec::new();
    rows.try_reserve_exact(count)
        .map_err(|_| Error::Allocation)?;
    by_id
        .try_reserve_exact(count)
        .map_err(|_| Error::Allocation)?;
    let resident_bytes = add(capacity(&rows)?, capacity(&by_id)?)?;
    if resident_bytes > max_bytes {
        return Err(Error::Limit.into());
    }
    collector
        .reserve_external(resident_bytes.checked_sub(requested).ok_or(Error::State)?)
        .map_err(Failure::Collector)?;
    for map in maps.all() {
        by_id.push(rows.len());
        rows.push(TopologyEntry {
            id: map.id(),
            parent_map_id: map.parent_map_id(),
            wdt_file_data_id: map.wdt_file_data_id(),
            physical_ordinal: map.physical_ordinal(),
            depth: map.depth(),
            is_parent: map.is_parent(),
        });
    }
    by_id.sort_unstable_by_key(|&i| rows[i].id);
    Ok(Topology {
        rows,
        by_id,
        direct_count: maps.stats().direct_rows,
        resident_bytes,
    })
}
pub(super) fn prepare<A: Access>(
    access: &mut A,
    limits: Limits,
) -> Result<Prepared, Failure<A::Error>> {
    let (snapshot, mut map_facts) = decode(
        access,
        MAP_FILE_DATA_ID,
        true,
        limits,
        source::read_vmap_maps,
    )?;
    for row in snapshot.direct_records() {
        if row.fields.name.len() > limits.maps.max_name_bytes
            || row.fields.directory.len() > limits.maps.max_name_bytes
        {
            return Err(Error::Limit.into());
        }
    }
    let direct_count = snapshot.direct_records().len();
    // The snapshot and materializer's index/growth/cloned strings remain under
    // the same allowance until materialize returns and destroys its scratch.
    let rows = snapshot.materialize().map_err(Error::Db2)?;
    let mut retained = capacity(&rows)?;
    for row in &rows {
        retained = add(
            retained,
            add(row.fields.name.capacity(), row.fields.directory.capacity())?,
        )?;
    }
    if rows.len() > limits.maps.max_rows
        || retained > limits.max_table_memory_bytes
        || retained > map_facts.decode_allowance_bytes
    {
        return Err(Error::Limit.into());
    }
    access
        .release_external(map_facts.decode_allowance_bytes - retained)
        .map_err(Failure::Collector)?;
    map_facts.materialized_rows = rows.len();
    map_facts.retained_bytes = retained;

    let (snapshot, mut material_facts) = decode(
        access,
        MATERIAL_FILE_DATA_ID,
        false,
        limits,
        source::read_liquid_materials,
    )?;
    let materials = replay(access, &snapshot, limits, retained)?;
    material_facts.materialized_rows = materials.len();
    material_facts.retained_bytes = capacity(&materials)?;
    retained = add(retained, material_facts.retained_bytes)?;
    drop(snapshot);
    access
        .release_external(material_facts.decode_allowance_bytes)
        .map_err(Failure::Collector)?;

    let (snapshot, mut type_facts) = decode(
        access,
        TYPE_FILE_DATA_ID,
        true,
        limits,
        source::read_liquid_types,
    )?;
    let types = replay(access, &snapshot, limits, retained)?;
    type_facts.materialized_rows = types.len();
    type_facts.retained_bytes = capacity(&types)?;
    retained = add(retained, type_facts.retained_bytes)?;
    drop(snapshot);
    access
        .release_external(type_facts.decode_allowance_bytes)
        .map_err(Failure::Collector)?;
    Ok(Prepared {
        rows,
        direct_count,
        lookup: Lookups { materials, types },
        tables: [map_facts, material_facts, type_facts],
        retained,
    })
}
fn decode<T, A: Access>(
    access: &mut A,
    id: u32,
    text: bool,
    limits: Limits,
    reader: fn(&[u8]) -> Result<source::SourceRecords<T>, RecordError>,
) -> Result<(source::SourceRecords<T>, TableFacts), Failure<A::Error>> {
    let (bytes, facts) = access
        .read_source(id, limits.max_db2_bytes)
        .map_err(Failure::Collector)?;
    if word(&bytes, 136).is_some_and(|n| n as usize > limits.max_direct_rows)
        || (word(&bytes, 200) == Some(1)
            && word(&bytes, 240).is_some_and(|n| n as usize > limits.max_copy_records))
    {
        return Err(Error::Limit.into());
    }
    let allowance = add(
        add(
            bytes.len().checked_mul(32).ok_or(Error::Limit)?,
            2 * 1024 * 1024,
        )?,
        if text { TEXT_ALLOWANCE } else { 0 },
    )?;
    if allowance > limits.max_decode_allowance_bytes {
        return Err(Error::Limit.into());
    }
    access
        .reserve_external(allowance)
        .map_err(Failure::Collector)?;
    let snapshot = reader(&bytes).map_err(Error::Db2)?;
    let direct_rows = snapshot.direct_records().len();
    let copy_records = snapshot.copies().len();
    if direct_rows > limits.max_direct_rows || copy_records > limits.max_copy_records {
        return Err(Error::Limit.into());
    }
    let source_capacity = bytes.capacity();
    drop(bytes);
    access
        .release_source(source_capacity)
        .map_err(Failure::Collector)?;
    Ok((
        snapshot,
        TableFacts {
            source: facts,
            direct_rows,
            copy_records,
            materialized_rows: 0,
            retained_bytes: 0,
            decode_allowance_bytes: allowance,
        },
    ))
}
/// Precollect every eventual key, then replay against a sorted vector. Default
/// entries are logically inserted only when observed by copies; preinitializing
/// them is equivalent because direct assignment runs first and no queries occur
/// until replay ends. Retained capacity includes duplicate-key scratch slots.
pub(super) fn replay<T: Copy + Default, A: Access>(
    access: &mut A,
    snapshot: &source::SourceRecords<T>,
    limits: Limits,
    retained: usize,
) -> Result<Vec<(u32, T)>, Failure<A::Error>> {
    let count = add(
        snapshot.direct_records().len(),
        snapshot.copies().len().checked_mul(2).ok_or(Error::Limit)?,
    )?;
    if count > MAX_LOOKUP_ROWS {
        return Err(Error::Limit.into());
    }
    let requested = count
        .checked_mul(size_of::<(u32, T)>())
        .ok_or(Error::Limit)?;
    if add(retained, requested)? > limits.max_table_memory_bytes {
        return Err(Error::Limit.into());
    }
    access
        .reserve_external(requested)
        .map_err(Failure::Collector)?;
    let mut rows = Vec::new();
    rows.try_reserve_exact(count)
        .map_err(|_| Error::Allocation)?;
    let actual = capacity(&rows)?;
    if actual > requested {
        access
            .reserve_external(actual - requested)
            .map_err(Failure::Collector)?;
    }
    if add(retained, actual)? > limits.max_table_memory_bytes {
        return Err(Error::Limit.into());
    }
    rows.extend(
        snapshot
            .direct_records()
            .iter()
            .map(|r| (r.id, T::default())),
    );
    for copy in snapshot.copies() {
        rows.push((copy.source_id, T::default()));
        rows.push((copy.new_id, T::default()));
    }
    rows.sort_unstable_by_key(|row| row.0);
    rows.dedup_by_key(|row| row.0);
    if rows.len() > limits.max_lookup_rows {
        return Err(Error::Limit.into());
    }
    for row in snapshot.direct_records() {
        let index = rows
            .binary_search_by_key(&row.id, |r| r.0)
            .map_err(|_| Error::State)?;
        rows[index].1 = row.fields;
    }
    for copy in snapshot.copies() {
        let source = rows
            .binary_search_by_key(&copy.source_id, |r| r.0)
            .map_err(|_| Error::State)?;
        let dest = rows
            .binary_search_by_key(&copy.new_id, |r| r.0)
            .map_err(|_| Error::State)?;
        rows[dest].1 = rows[source].1;
    }
    Ok(rows)
}
