// SPDX-License-Identifier: GPL-3.0-or-later
// Authored 2026-10-11: owned route tables and ordinal indexes; no self-references.

use super::{Budget, Error, add, array_bytes, reserve};
use crate::{
    Locale,
    build_config::{AuxiliaryKind, BuildConfig, ContentReference},
    encoding, root,
};
use std::{cmp::Ordering, ops::Range};

pub(super) fn config_references(config: &BuildConfig<'_>) -> Result<Vec<ContentReference>, Error> {
    let mut auxiliary = Vec::new();
    reserve(&mut auxiliary, config.auxiliary.len())?;
    auxiliary.extend(
        config
            .auxiliary
            .iter()
            .filter(|entry| entry.content_key.is_some())
            .copied(),
    );
    auxiliary.sort_unstable_by_key(|entry| entry.kind);
    let mut refs = Vec::new();
    reserve(&mut refs, add(auxiliary.len(), 2)?)?;
    refs.push(config.encoding); // Inserted before ENCODING rows by the native loader.
    for entry in auxiliary
        .iter()
        .filter(|entry| entry.kind < AuxiliaryKind::Size)
    {
        refs.push(content(entry)?);
    }
    refs.push(config.root);
    for entry in auxiliary
        .iter()
        .filter(|entry| entry.kind >= AuxiliaryKind::Size)
    {
        refs.push(content(entry)?);
    }
    Ok(refs)
}

fn content(entry: &crate::build_config::AuxiliaryReference) -> Result<ContentReference, Error> {
    Ok(ContentReference {
        content_key: entry.content_key.ok_or(Error::BuildMetadata)?,
        encoded_key: entry.encoded_key,
        content_size: entry.content_size,
        encoded_size: entry.encoded_size,
    })
}

#[derive(Clone, Copy)]
struct Route {
    key: [u8; 16],
    size: Option<u32>,
    start: u32,
    count: u32,
    build_reference: bool,
}

pub(super) struct Central {
    rows: Vec<Route>,
    keys: Vec<[u8; 16]>,
    order: Vec<u32>,
}
impl Central {
    pub(super) fn from_routes(
        routes: encoding::EncodingRoutes<'_>,
        refs: &[ContentReference],
        budget: &mut Budget,
    ) -> Result<Self, Error> {
        let row_count = add(routes.entries.len(), refs.len())?;
        let keys_count = routes.entries.iter().try_fold(
            refs.iter().filter(|r| r.encoded_key.is_some()).count(),
            |count, row| add(count, row.encoded_keys.len()),
        )?;
        if row_count > u32::MAX as usize || keys_count > u32::MAX as usize {
            return Err(Error::Limit);
        }
        budget.retain(array_bytes::<Route>(row_count)?)?;
        budget.retain(array_bytes::<[u8; 16]>(keys_count)?)?;
        let mut rows = Vec::new();
        let mut keys = Vec::new();
        reserve(&mut rows, row_count)?;
        reserve(&mut keys, keys_count)?;
        let mut append = |key, size, values: &[[u8; 16]], build_reference| {
            rows.push(Route {
                key,
                size,
                start: keys.len() as u32,
                count: values.len() as u32,
                build_reference,
            });
            keys.extend_from_slice(values);
        };
        if let Some(first) = refs.first() {
            append(
                first.content_key,
                first.content_size,
                first.encoded_key.as_slice(),
                true,
            );
        }
        for row in &routes.entries {
            append(row.content_key, row.content_size, row.encoded_keys, false);
        }
        for reference in refs.iter().skip(1) {
            append(
                reference.content_key,
                reference.content_size,
                reference.encoded_key.as_slice(),
                true,
            );
        }
        drop(routes);
        let order = sorted_order(rows.len(), budget, |a, b| rows[a].key.cmp(&rows[b].key))?;
        Ok(Self { rows, keys, order })
    }

    fn matching(&self, key: [u8; 16]) -> impl Iterator<Item = &Route> {
        let start = self
            .order
            .partition_point(|&index| self.rows[index as usize].key < key);
        self.order[start..]
            .iter()
            .map(|&index| &self.rows[index as usize])
            .take_while(move |row| row.key == key)
    }
    pub(super) fn contains(&self, key: [u8; 16]) -> bool {
        self.matching(key).next().is_some()
    }
    pub(super) fn content_size(&self, key: [u8; 16]) -> Option<u32> {
        // Map::InsertObject keeps the first ENCODING row. Only the overload
        // for build references can fill its unknown size (CascOpenStorage.cpp
        // 172-243, common/Map.h 181-186 at the pinned reference).
        let mut rows = self.matching(key);
        rows.next()?.size.or_else(|| {
            rows.filter(|row| row.build_reference)
                .find_map(|row| row.size)
        })
    }
    pub(super) fn encoded_keys(&self, key: [u8; 16]) -> impl Iterator<Item = [u8; 16]> {
        self.matching(key).flat_map(|row| {
            self.keys[row.start as usize..row.start as usize + row.count as usize]
                .iter()
                .copied()
        })
    }
}

pub(super) struct OwnedRoot {
    pub(super) entries: Vec<root::Entry>,
    blocks: Vec<root::Block>,
    order: Vec<u32>,
    // Metadata is deliberately owned even though lookups do not need it.
    _layout: root::Layout,
    _total: Option<u32>,
    _named: Option<u32>,
    _extension: Vec<u8>,
}
impl OwnedRoot {
    pub(super) fn blocks(&self) -> &[root::Block] {
        &self.blocks
    }

    pub(super) fn from_root(parsed: root::Root<'_>, budget: &mut Budget) -> Result<Self, Error> {
        let header = parsed.header;
        budget.retain(header.extension.len())?;
        let mut extension = Vec::new();
        reserve(&mut extension, header.extension.len())?;
        extension.extend_from_slice(header.extension);
        let (blocks, entries) = parsed.into_parts();
        budget.retain(array_bytes::<root::Block>(blocks.capacity())?)?;
        budget.retain(array_bytes::<root::Entry>(entries.capacity())?)?;
        let order = sorted_order(entries.len(), budget, |a, b| {
            let ea = &entries[a];
            let eb = &entries[b];
            let aa = blocks[ea.block_index].content_flags.effective() >> 31;
            let ab = blocks[eb.block_index].content_flags.effective() >> 31;
            (ea.file_data_id, aa).cmp(&(eb.file_data_id, ab))
        })?;
        Ok(Self {
            entries,
            blocks,
            order,
            _layout: header.layout,
            _total: header.total_files,
            _named: header.files_with_name_hash,
            _extension: extension,
        })
    }
    pub(super) fn resolve(
        &self,
        id: u32,
        locale: Locale,
        mut present: impl FnMut([u8; 16]) -> bool,
    ) -> Option<&root::Entry> {
        self.eligible(id, locale)
            .find(|entry| present(entry.content_key))
    }

    // Shared eligibility/order for acquisition and key-free diagnostics. A
    // missing archive never changes which central-present ROOT row is chosen.
    pub(super) fn eligible(&self, id: u32, locale: Locale) -> impl Iterator<Item = &root::Entry> {
        let mask = match locale {
            Locale::EnUs => 2,
            Locale::EsEs => 0x80,
        };
        self.matching(id).filter(move |entry| {
            let block = &self.blocks[entry.block_index];
            let flags = block.content_flags.effective();
            flags & (0x100 | 0x80) == 0
                && (block.locale_flags == 0 || block.locale_flags & mask != 0)
        })
    }

    pub(super) fn matching(&self, id: u32) -> impl Iterator<Item = &root::Entry> {
        let range = equal_range(&self.order, |index| {
            self.entries[index].file_data_id.cmp(&id)
        });
        self.order[range]
            .iter()
            .map(|&index| &self.entries[index as usize])
    }
}

pub(super) fn equal_range(
    order: &[u32],
    mut compare: impl FnMut(usize) -> Ordering,
) -> Range<usize> {
    let start = order.partition_point(|&index| compare(index as usize) == Ordering::Less);
    let end =
        start + order[start..].partition_point(|&index| compare(index as usize) == Ordering::Equal);
    start..end
}

pub(super) fn sorted_order(
    count: usize,
    budget: &mut Budget,
    mut compare: impl FnMut(usize, usize) -> Ordering,
) -> Result<Vec<u32>, Error> {
    if count > u32::MAX as usize {
        return Err(Error::Limit);
    }
    budget.retain(array_bytes::<u32>(count)?)?;
    let mut order = Vec::new();
    reserve(&mut order, count)?;
    order.extend((0..count).map(|index| index as u32));
    order.sort_unstable_by(|&a, &b| compare(a as usize, b as usize).then(a.cmp(&b)));
    Ok(order)
}
