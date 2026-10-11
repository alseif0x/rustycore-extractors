// SPDX-License-Identifier: GPL-3.0-or-later
// Authored 2026-10-11: original navigation direct-row LiquidType reduction.
//! Last direct ID wins, including ID zero. Ordered DB2 copy records are ignored,
//! as in PathGenerator's GetRecordCount/GetRecord startup loop. MaterialID is
//! unrelated to this navigation callback. Acquisition remains caller-owned.
use super::LiquidTypes;
use crate::{Error, reserve};
use forever_db2::terrain_sources::{DirectRecord, LiquidTypeFields, SourceRecords};
#[derive(Clone, Copy, Debug)]
pub struct Limits {
    pub rows: usize,
    pub entries: usize,
    /// Actual index/output Vec capacities, including their overlap. Borrowed
    /// source rows and allocator overhead are excluded. No extra sort heap.
    pub heap_bytes: usize,
}
impl Default for Limits {
    fn default() -> Self {
        Self {
            rows: 200_000,
            entries: 65_536,
            heap_bytes: 2 * 1024 * 1024,
        }
    }
}
pub struct Table {
    entries: Vec<(u32, u8)>,
}
impl Table {
    pub fn from_source(
        source: &SourceRecords<LiquidTypeFields>,
        limits: Limits,
    ) -> Result<Self, Error> {
        Self::from_rows(source.direct_records(), limits)
    }
    pub fn from_rows(
        rows: &[DirectRecord<LiquidTypeFields>],
        limits: Limits,
    ) -> Result<Self, Error> {
        let d = Limits::default();
        if limits.rows > d.rows
            || limits.entries > d.entries
            || limits.heap_bytes > d.heap_bytes
            || rows.len() > limits.rows
            || rows.len() * 4 > limits.heap_bytes
        {
            return Err(Error::Limit);
        }
        let mut order = Vec::<u32>::new();
        reserve(&mut order, rows.len())?;
        if order.capacity() * 4 > limits.heap_bytes {
            return Err(Error::Limit);
        }
        order.extend(0..rows.len() as u32);
        // Ordinal is part of the unstable sort key, retaining the last original
        // direct occurrence without any stable-sort scratch allocation.
        order.sort_unstable_by_key(|&i| (rows[i as usize].id, i));
        let count = order
            .iter()
            .enumerate()
            .filter(|&(at, &i)| {
                at + 1 == order.len() || rows[i as usize].id != rows[order[at + 1] as usize].id
            })
            .count();
        if count > limits.entries || order.capacity() * 4 + count * 8 > limits.heap_bytes {
            return Err(Error::Limit);
        }
        let mut entries = Vec::new();
        reserve(&mut entries, count)?;
        if order.capacity() * 4 + entries.capacity() * 8 > limits.heap_bytes {
            return Err(Error::Limit);
        }
        for (at, &i) in order.iter().enumerate() {
            let row = &rows[i as usize];
            if at + 1 == order.len() || row.id != rows[order[at + 1] as usize].id {
                if row.fields.sound_bank >= 31 {
                    return Err(Error::LiquidTypes);
                }
                entries.push((row.id, row.fields.sound_bank));
            }
        }
        Ok(Self { entries })
    }
    pub fn entries(&self) -> &[(u32, u8)] {
        &self.entries
    }
    pub fn types(&self) -> LiquidTypes<'_> {
        LiquidTypes {
            entries: &self.entries,
        }
    }
    pub fn capacity_bytes(&self) -> usize {
        self.entries.capacity() * 8
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    fn row(id: u32, bank: u8) -> DirectRecord<LiquidTypeFields> {
        DirectRecord {
            id,
            fields: LiquidTypeFields {
                sound_bank: bank,
                material_id: 255,
            },
        }
    }
    #[test]
    fn sorted_unique_last_direct_id_wins_including_zero() {
        let rows = [row(7, 3), row(0, 4), row(7, 1), row(100, 30), row(0, 2)];
        let t = Table::from_rows(&rows, Limits::default()).unwrap();
        assert_eq!(t.entries(), [(0, 2), (7, 1), (100, 30)]);
        assert_eq!(t.types().flags(0), 4);
        assert_eq!(t.types().flags(7), 2);
        assert_eq!(t.types().flags(100), 0);
        assert_eq!(t.types().flags(999), 0);
    }
    #[test]
    fn overwritten_bad_bank_is_not_shifted_but_final_bad_bank_rejects() {
        assert!(Table::from_rows(&[row(1, 255), row(1, 0)], Limits::default()).is_ok());
        assert!(matches!(
            Table::from_rows(&[row(1, 0), row(1, 31)], Limits::default()),
            Err(Error::LiquidTypes)
        ));
    }
    #[test]
    fn only_downward_and_exact_capacity_admission() {
        let rows = [row(1, 0), row(1, 2), row(2, 3)];
        let l = Limits {
            rows: 3,
            entries: 2,
            heap_bytes: 28,
        };
        let t = Table::from_rows(&rows, l).unwrap();
        assert_eq!(t.capacity_bytes(), 16);
        for limits in [
            Limits {
                heap_bytes: 27,
                ..l
            },
            Limits { entries: 1, ..l },
            Limits { rows: 2, ..l },
            Limits { rows: 200001, ..l },
            Limits {
                entries: 65537,
                ..l
            },
            Limits {
                heap_bytes: 2 * 1024 * 1024 + 1,
                ..l
            },
        ] {
            assert!(matches!(Table::from_rows(&rows, limits), Err(Error::Limit)));
        }
    }
    #[test]
    fn empty_zero_budget_and_maximum_direct_duplicate_count() {
        assert!(
            Table::from_rows(
                &[],
                Limits {
                    rows: 0,
                    entries: 0,
                    heap_bytes: 0
                }
            )
            .unwrap()
            .entries()
            .is_empty()
        );
        let mut rows = vec![row(1, 0); 200000];
        rows[199999] = row(1, 30);
        let t = Table::from_rows(&rows, Limits::default()).unwrap();
        assert_eq!(t.entries(), [(1, 30)]);
        rows.push(row(2, 0));
        assert!(matches!(
            Table::from_rows(&rows, Limits::default()),
            Err(Error::Limit)
        ));
    }
}
