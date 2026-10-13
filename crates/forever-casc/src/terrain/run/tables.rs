// SPDX-License-Identifier: GPL-3.0-or-later
// Authored 2026-10-11: complete map-extractor startup projections and accounting.
use super::*;
use std::{collections::BTreeMap, mem::size_of};

pub(super) struct Tables {
    pub maps: Vec<source::DirectRecord<source::MapFields>>,
    materials: Vec<(u32, source::LiquidMaterialFields)>,
    types: Vec<(u32, source::LiquidTypeFields)>,
    pub facts: [TableFacts; 4],
}
impl liquid::LiquidLookup for Tables {
    fn liquid_type(&self, id: u32) -> Option<liquid::LiquidType> {
        let i = self.types.binary_search_by_key(&id, |r| r.0).ok()?;
        let v = self.types[i].1;
        Some(liquid::LiquidType {
            sound_bank: v.sound_bank,
            material_id: v.material_id,
        })
    }
    fn material(&self, id: u32) -> Option<liquid::LiquidMaterial> {
        let i = self.materials.binary_search_by_key(&id, |r| r.0).ok()?;
        let v = self.materials[i].1;
        Some(liquid::LiquidMaterial {
            flags: v.flags_bits,
            vertex_format: v.lvf,
        })
    }
}
impl<S: Sink> Runner<'_, S> {
    pub(super) fn tables(&mut self) -> Result<Tables, Error> {
        let (maps, map_facts) = self.map_table()?;
        let (materials, material_facts) = self.material_table()?;
        // The pinned startup loads/replays this table. Its values are unused by
        // conversion, so release them after validation rather than retaining a
        // second unused lookup for the entire extraction.
        let object_facts = self.object_table()?;
        let (types, type_facts) = self.type_table()?;
        Ok(Tables {
            maps,
            materials,
            types,
            facts: [map_facts, material_facts, object_facts, type_facts],
        })
    }
    fn map_table(
        &mut self,
    ) -> Result<(Vec<source::DirectRecord<source::MapFields>>, TableFacts), Error> {
        let file = self.table_file(Step::Map, MAP_ID, true)?;
        let rows = source::read_map(&file.bytes).map_err(Error::Db2)?;
        let mut facts = self.table_facts(&file, &rows)?;
        let maps = rows.materialize().map_err(Error::Db2)?;
        facts.materialized_rows = maps.len();
        let mut bytes = capacity(&maps)?;
        for row in &maps {
            bytes = add(
                bytes,
                add(row.fields.name.capacity(), row.fields.directory.capacity())?,
            )?;
        }
        self.retain(bytes)?;
        Ok((maps, facts))
    }
    fn material_table(
        &mut self,
    ) -> Result<(Vec<(u32, source::LiquidMaterialFields)>, TableFacts), Error> {
        let file = self.table_file(Step::Material, MATERIAL_ID, false)?;
        let rows = source::read_liquid_materials(&file.bytes).map_err(Error::Db2)?;
        let mut facts = self.table_facts(&file, &rows)?;
        let rows = compact(
            rows.materialize().map_err(Error::Db2)?,
            self.limits.max_lookup_rows,
        )?;
        facts.materialized_rows = rows.len();
        self.retain(capacity(&rows)?)?;
        Ok((rows, facts))
    }
    fn object_table(&mut self) -> Result<TableFacts, Error> {
        let file = self.table_file(Step::Object, OBJECT_ID, false)?;
        let rows = source::read_liquid_objects(&file.bytes).map_err(Error::Db2)?;
        let mut facts = self.table_facts(&file, &rows)?;
        let rows = rows.materialize().map_err(Error::Db2)?;
        if rows.len() > self.limits.max_lookup_rows {
            return Err(Error::Limit);
        }
        facts.materialized_rows = rows.len();
        Ok(facts)
    }
    fn type_table(&mut self) -> Result<(Vec<(u32, source::LiquidTypeFields)>, TableFacts), Error> {
        let file = self.table_file(Step::Type, TYPE_ID, true)?;
        let rows = source::read_liquid_types(&file.bytes).map_err(Error::Db2)?;
        let mut facts = self.table_facts(&file, &rows)?;
        let rows = compact(
            rows.materialize().map_err(Error::Db2)?,
            self.limits.max_lookup_rows,
        )?;
        facts.materialized_rows = rows.len();
        self.retain(capacity(&rows)?)?;
        Ok((rows, facts))
    }
    fn table_file(&mut self, step: Step, id: u32, text: bool) -> Result<VerifiedFile, Error> {
        self.step = step;
        let file = self.read(Some(id), &[], self.limits.max_db2_bytes, self.retained)?;
        let allowance = add(
            2 * MIB + if text { 64 * MIB } else { 0 },
            file.bytes.len().checked_mul(32).ok_or(Error::Limit)?,
        )?;
        if allowance > self.limits.max_decode_allowance {
            return Err(Error::Limit);
        }
        self.memory(add(self.retained, add(file.bytes.capacity(), allowance)?)?)?;
        Ok(file)
    }
    fn table_facts<T>(
        &self,
        file: &VerifiedFile,
        rows: &source::SourceRecords<T>,
    ) -> Result<TableFacts, Error> {
        if rows.direct_records().len() > self.limits.max_direct_rows
            || rows.copies().len() > self.limits.max_copy_records
        {
            return Err(Error::Limit);
        }
        Ok(TableFacts {
            source: self.facts(file),
            direct_rows: rows.direct_records().len(),
            copy_records: rows.copies().len(),
            materialized_rows: 0,
        })
    }
    fn retain(&mut self, bytes: usize) -> Result<(), Error> {
        let total = add(self.retained, bytes)?;
        if total > self.limits.max_table_bytes {
            return Err(Error::Limit);
        }
        self.memory(total)?;
        self.retained = total;
        Ok(())
    }
}
fn capacity<T>(rows: &Vec<T>) -> Result<usize, Error> {
    rows.capacity()
        .checked_mul(size_of::<T>())
        .ok_or(Error::Limit)
}
fn compact<T>(tree: BTreeMap<u32, T>, limit: usize) -> Result<Vec<(u32, T)>, Error> {
    if tree.len() > limit {
        return Err(Error::Limit);
    }
    let mut rows = Vec::new();
    rows.try_reserve_exact(tree.len())
        .map_err(|_| Error::Allocation)?;
    rows.extend(tree);
    Ok(rows)
}
