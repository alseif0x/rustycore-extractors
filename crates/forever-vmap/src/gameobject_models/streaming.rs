// SPDX-License-Identifier: GPL-3.0-or-later
// Authored 2026-10-11: original pure row-wise GameObject list construction.
//! Complete metadata preflight, private provisional lists, one borrowed RAW per
//! row. No all-row RAW or bounds cache, I/O, keys or implicit model resolution.
//! Names/IDs live in the provisional outputs; the compact row plan holds only
//! class, RAW length and bound offset. Vertex quotas are charged during parsing.

use super::{Error as CodecError, Limits, Lists, RawModel, Stats, add, push};

pub const PROFILE: &str = "gameobject-model-lists-rowwise-v1";
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Class {
    M2,
    Wmo,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ModelInfo {
    pub class: Class,
    pub raw_bytes: usize,
}
#[derive(Clone, Copy)]
pub struct Metadata<'a> {
    pub display_id: u32,
    pub name: &'a [u8],
    pub model: Option<ModelInfo>,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Error {
    Codec(CodecError),
    State,
}
impl Error {
    pub const fn code(self) -> &'static str {
        match self {
            Self::Codec(e) => e.code(),
            Self::State => "gameobject_streaming_state",
        }
    }
}
impl From<CodecError> for Error {
    fn from(e: CodecError) -> Self {
        Self::Codec(e)
    }
}
impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.code())
    }
}
impl std::error::Error for Error {}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Plan {
    rows: usize,
    temp: usize,
    dtree: usize,
    raw_visits: u64,
    planned: usize,
}
impl Plan {
    /// No allocations or RAW parsing. All names/lengths and repeated RAW visits
    /// are admitted before Builder allocates anything. Model content is not proof.
    pub fn preflight(rows: &[Metadata<'_>], limits: Limits) -> Result<Self, Error> {
        limits.validate()?;
        if rows.len() > limits.max_rows {
            return Err(CodecError::Limit.into());
        }
        let (mut temp, mut dtree, mut visits) = (8usize, 8usize, 0u64);
        for row in rows {
            super::name(row.name, limits)?;
            let model = row.model.ok_or(CodecError::MissingModel)?;
            if model.raw_bytes > limits.max_raw_bytes {
                return Err(CodecError::Limit.into());
            }
            visits = visits
                .checked_add(u64::try_from(model.raw_bytes).map_err(|_| CodecError::Limit)?)
                .ok_or(CodecError::Limit)?;
            if visits > limits.max_raw_visit_bytes {
                return Err(CodecError::Limit.into());
            }
            let length = add(8, row.name.len())?;
            temp = add(temp, length)?;
            dtree = add(dtree, add(length, 24)?)?;
            if temp > limits.max_list_bytes || dtree > limits.max_list_bytes {
                return Err(CodecError::Limit.into());
            }
        }
        if temp > limits.max_list_bytes || dtree > limits.max_list_bytes {
            return Err(CodecError::Limit.into());
        }
        let entries = rows
            .len()
            .checked_mul(size_of::<Entry>())
            .ok_or(CodecError::Limit)?;
        let planned = add(add(temp, dtree)?, entries)?;
        if planned > limits.max_memory_bytes {
            return Err(CodecError::Limit.into());
        }
        Ok(Self {
            rows: rows.len(),
            temp,
            dtree,
            raw_visits: visits,
            planned,
        })
    }
    pub const fn planned_bytes(self) -> usize {
        self.planned
    }
    pub const fn rows(self) -> usize {
        self.rows
    }
    pub const fn raw_visit_bytes(self) -> u64 {
        self.raw_visits
    }
}
struct Entry {
    model: ModelInfo,
    bound_offset: usize,
}
/// No output access until finish succeeds. Failure is sticky; no row replay or
/// partial Lists is exposed. Buffers reserve once with full capacity accounting.
pub struct Builder {
    entries: Vec<Entry>,
    temp: Vec<u8>,
    dtree: Vec<u8>,
    next: usize,
    limits: Limits,
    stats: Stats,
    failed: Option<Error>,
}
impl Builder {
    pub fn new(plan: Plan, rows: &[Metadata<'_>], limits: Limits) -> Result<Self, Error> {
        if Plan::preflight(rows, limits)? != plan {
            return Err(Error::State);
        }
        let mut memory = super::Memory {
            live: 0,
            peak: 0,
            cap: limits.max_memory_bytes,
        };
        let (mut entries, mut temp, mut dtree) = (Vec::new(), Vec::new(), Vec::new());
        memory.reserve(&mut entries, plan.rows)?;
        memory.reserve(&mut temp, plan.temp)?;
        memory.reserve(&mut dtree, plan.dtree)?;
        if temp.capacity() > limits.max_list_bytes || dtree.capacity() > limits.max_list_bytes {
            return Err(CodecError::Limit.into());
        }
        temp.extend_from_slice(super::TEMP_MAGIC);
        dtree.extend_from_slice(super::DTREE_MAGIC);
        for row in rows {
            for out in [&mut temp, &mut dtree] {
                push(out, row.display_id);
                push(
                    out,
                    u32::try_from(row.name.len()).map_err(|_| CodecError::Limit)?,
                );
                out.extend_from_slice(row.name)
            }
            let bound_offset = dtree.len();
            dtree.resize(add(bound_offset, 24)?, 0);
            entries.push(Entry {
                model: row.model.ok_or(CodecError::MissingModel)?,
                bound_offset,
            });
        }
        if temp.len() != plan.temp || dtree.len() != plan.dtree {
            return Err(Error::State);
        }
        Ok(Self {
            entries,
            temp,
            dtree,
            next: 0,
            limits,
            stats: Stats {
                rows: plan.rows,
                temp_bytes: plan.temp,
                dtree_bytes: plan.dtree,
                raw_visit_bytes: plan.raw_visits,
                peak_bytes: memory.peak,
                ..Stats::default()
            },
            failed: None,
        })
    }
    /// Actual output+plan capacities, while a caller's current RAW is excluded.
    pub fn resident_bytes(&self) -> Result<usize, Error> {
        Ok(add(
            add(self.temp.capacity(), self.dtree.capacity())?,
            self.entries
                .capacity()
                .checked_mul(size_of::<Entry>())
                .ok_or(CodecError::Limit)?,
        )?)
    }
    pub fn push(&mut self, model: RawModel<'_>) -> Result<(), Error> {
        if let Some(e) = self.failed {
            return Err(e);
        }
        let result = self.push_inner(model);
        if let Err(e) = result {
            self.failed = Some(e)
        }
        result
    }
    fn push_inner(&mut self, model: RawModel<'_>) -> Result<(), Error> {
        let entry = self.entries.get(self.next).ok_or(Error::State)?;
        let mut bound = None;
        match (entry.model.class, model) {
            (Class::M2, RawModel::M2(bytes)) => {
                if bytes.len() != entry.model.raw_bytes {
                    return Err(Error::State);
                }
                let raw = crate::RawM2::parse(
                    bytes,
                    crate::Limits {
                        max_input_bytes: self.limits.max_raw_bytes,
                        ..crate::Limits::default()
                    },
                )
                .map_err(CodecError::RawM2)?;
                super::charge_visits(&mut self.stats, raw.vertex_count(), self.limits)?;
                for index in 0..raw.vertex_count() {
                    super::merge(
                        &mut bound,
                        raw.vertex_bits(index).ok_or(CodecError::Framing)?,
                    )?
                }
                self.stats.m2_rows += 1;
            }
            (Class::Wmo, RawModel::Wmo(bytes)) => {
                if bytes.len() != entry.model.raw_bytes {
                    return Err(Error::State);
                }
                let raw = crate::wmo::raw::RawWmo::parse(
                    bytes,
                    crate::wmo::raw::ConversionLimits {
                        max_raw_bytes: self.limits.max_raw_bytes,
                        ..crate::wmo::raw::ConversionLimits::default()
                    },
                )
                .map_err(CodecError::RawWmo)?;
                for group in raw.groups() {
                    super::charge_visits(&mut self.stats, group.vertex_count(), self.limits)?;
                    for index in 0..group.vertex_count() {
                        super::merge(
                            &mut bound,
                            group.vertex_bits(index).ok_or(CodecError::Framing)?,
                        )?
                    }
                }
                self.stats.wmo_rows += 1;
            }
            _ => return Err(Error::State),
        }
        let bounds = bound.ok_or(CodecError::EmptyVertices)?;
        for (i, bits) in bounds.into_iter().enumerate() {
            let at = entry.bound_offset + i * 4;
            self.dtree[at..at + 4].copy_from_slice(&bits.to_le_bytes())
        }
        self.next += 1;
        Ok(())
    }
    pub fn finish(mut self) -> Result<Lists, Error> {
        if let Some(e) = self.failed {
            return Err(e);
        }
        if self.next != self.entries.len() {
            return Err(Error::State);
        }
        super::TempList::parse(&self.temp, self.limits)?;
        super::Dtree::parse(&self.dtree, self.limits)?;
        self.stats.resident_bytes = add(self.temp.capacity(), self.dtree.capacity())?;
        Ok(Lists {
            temp: self.temp,
            dtree: self.dtree,
            stats: self.stats,
        })
    }
}
#[cfg(test)]
mod tests;
