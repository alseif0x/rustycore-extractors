// SPDX-License-Identifier: GPL-3.0-or-later
// Authored 2026-10-13: selected product domain from complete authenticated Map.
//! Retains compact facts only. All projections consume the same complete bytes;
//! neither a caller-supplied map list nor an output filename establishes ancestry.
use crate::Error;
use forever_casc::{collision::plan, integrity};
use forever_db2::terrain_sources as source;
use std::mem::size_of;

pub(crate) const MAX_DOMAIN_BYTES: usize = 64 * 1024;
pub(crate) const MAX_MAP_BYTES: usize = 16 * 1024 * 1024;
// Source + one sequential decoder allowance + graph maximum + compact facts.
// Conservative logical admission, excluding allocator overhead and process RSS.
#[cfg(target_os = "linux")]
pub(crate) const PREPARATION_BYTES: usize =
    MAX_MAP_BYTES + 66 * 1024 * 1024 + 32 * MAX_MAP_BYTES + 64 * 1024 * 1024 + MAX_DOMAIN_BYTES;

#[derive(Clone, Copy)]
struct Entry {
    id: u32,
    parent: Option<u32>,
    wdt: u32,
}

pub(crate) struct Domain {
    requested: u32,
    source_sha256: [u8; 32],
    entries: Vec<Entry>,
    ids: Vec<u32>,
}
impl Domain {
    pub(crate) fn derive(bytes: &[u8], requested: u32, navigation: bool) -> Result<Self, Error> {
        if bytes.len() > MAX_MAP_BYTES {
            return Err("product_map_domain_limit".into());
        }
        let snapshot = source::read_vmap_maps(bytes).map_err(|e| Error::from(e.code()))?;
        let direct = snapshot.direct_records().len();
        let rows = snapshot.materialize().map_err(|e| Error::from(e.code()))?;
        let maps = plan::Maps::from_materialized_selection(
            &rows,
            direct,
            plan::MapSelection::MapAndAncestors(requested),
            plan::Limits::default(),
        )
        .map_err(|e| Error::from(e.code()))?;
        let count = maps.preload().len();
        let mut entries = Vec::new();
        let mut ids = Vec::new();
        entries
            .try_reserve_exact(count)
            .map_err(|_| Error::from("product_allocation"))?;
        ids.try_reserve_exact(count)
            .map_err(|_| Error::from("product_allocation"))?;
        if compact_bytes(entries.capacity(), ids.capacity())? > MAX_DOMAIN_BYTES {
            return Err("product_map_domain_limit".into());
        }
        for map in maps.preload() {
            entries.push(Entry {
                id: map.id(),
                parent: map.parent_map_id(),
                wdt: map.wdt_file_data_id(),
            });
            ids.push(map.id());
        }
        drop(maps);
        drop(rows);
        // Terrain replays copies and removes WDT-zero rows. Admission requires
        // exactly the same selected identities; unselected rows stay validated.
        let terrain = source::read_map(bytes)
            .and_then(|table| table.materialize())
            .map_err(|e| Error::from(e.code()))?;
        for entry in &entries {
            let mut matching = terrain.iter().filter(|row| row.id == entry.id);
            let row = matching.next().ok_or("product_map_domain_projection")?;
            if matching.next().is_some() || row.fields.wdt_file_data_id as u32 != entry.wdt {
                return Err("product_map_domain_projection".into());
            }
        }
        drop(terrain);
        if navigation {
            let snapshot =
                source::read_navigation_maps(bytes).map_err(|e| Error::from(e.code()))?;
            for entry in &entries {
                let mut matching = snapshot
                    .direct_records()
                    .iter()
                    .filter(|row| row.id == entry.id);
                let row = matching.next().ok_or("product_map_domain_projection")?;
                let parent = row.fields.effective_parent();
                let parent = (parent >= 0).then_some(parent as u32);
                if matching.next().is_some() || parent != entry.parent {
                    return Err("product_map_domain_projection".into());
                }
            }
        }
        Ok(Self {
            requested,
            source_sha256: integrity::digest(bytes),
            entries,
            ids,
        })
    }
    pub(crate) fn ids(&self) -> &[u32] {
        &self.ids
    }
    pub(crate) const fn requested(&self) -> u32 {
        self.requested
    }
    pub(crate) fn contains(&self, id: u32) -> bool {
        self.ids.contains(&id)
    }
    #[cfg(target_os = "linux")]
    pub(crate) const fn selection(&self) -> plan::MapSelection {
        plan::MapSelection::MapAndAncestors(self.requested)
    }
    pub(crate) fn resident_bytes(&self) -> usize {
        // Both capacities were checked at construction and never grow.
        self.entries.capacity() * size_of::<Entry>() + self.ids.capacity() * size_of::<u32>()
    }
    /// A closed numeric view of original direct rows; both parent fields and
    /// all flags survive unchanged. No copy replay or synthetic SourceRecords.
    pub(crate) fn navigation_rows(
        &self,
        source: &source::SourceRecords<source::NavigationMapFields>,
    ) -> Result<Vec<source::DirectRecord<source::NavigationMapFields>>, Error> {
        let requested = self
            .ids
            .len()
            .checked_mul(size_of::<source::DirectRecord<source::NavigationMapFields>>())
            .and_then(|n| n.checked_add(self.resident_bytes()))
            .ok_or("product_map_domain_limit")?;
        if requested > MAX_DOMAIN_BYTES {
            return Err("product_map_domain_limit".into());
        }
        let mut rows: Vec<source::DirectRecord<source::NavigationMapFields>> = Vec::new();
        rows.try_reserve_exact(self.ids.len())
            .map_err(|_| Error::from("product_allocation"))?;
        self.navigation_resident_bytes(rows.capacity())?;
        for row in source.direct_records() {
            if !self.contains(row.id) {
                continue;
            }
            let entry = self
                .entries
                .iter()
                .find(|entry| entry.id == row.id)
                .ok_or("product_map_domain_projection")?;
            let parent = row.fields.effective_parent();
            let parent = (parent >= 0).then_some(parent as u32);
            if rows.iter().any(|existing| existing.id == row.id) || entry.parent != parent {
                return Err("product_map_domain_projection".into());
            }
            rows.push(row.clone());
        }
        if rows.len() != self.ids.len() {
            return Err("product_map_domain_projection".into());
        }
        Ok(rows)
    }
    pub(crate) fn navigation_resident_bytes(&self, capacity: usize) -> Result<usize, Error> {
        let bytes = capacity
            .checked_mul(size_of::<source::DirectRecord<source::NavigationMapFields>>())
            .and_then(|n| n.checked_add(self.resident_bytes()))
            .ok_or("product_map_domain_limit")?;
        if bytes > MAX_DOMAIN_BYTES {
            return Err("product_map_domain_limit".into());
        }
        Ok(bytes)
    }
    pub(crate) fn verify_source(&self, bytes: &[u8]) -> Result<(), Error> {
        if integrity::digest(bytes) != self.source_sha256 {
            return Err("product_map_domain_changed".into());
        }
        Ok(())
    }
}
fn compact_bytes(entries: usize, ids: usize) -> Result<usize, Error> {
    entries
        .checked_mul(size_of::<Entry>())
        .and_then(|n| {
            ids.checked_mul(size_of::<u32>())
                .and_then(|m| n.checked_add(m))
        })
        .ok_or_else(|| Error::from("product_map_domain_limit"))
}

#[cfg(test)]
pub(crate) mod tests;
