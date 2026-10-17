// SPDX-License-Identifier: GPL-3.0-or-later
// Authored by RustyCore Extractors, 2026-10-17.
//! Observations captured inside one genuine serial compatible world run.
//! Missing WDT and omitted available sections are unknown, never empty terrain.
//! No public constructor accepts reports, ancestry or caller supplied witnesses.
use crate::collision::{available_70374 as available, compatible::startup, plan, world};
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Wdt {
    Missing,
    Present,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Own {
    Unlisted,
    Missing,
    Empty,
    Output,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Replay {
    None,
    Empty,
    Output,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Tile {
    own: Own,
    replay: Replay,
    parent: Option<u32>,
}
impl Tile {
    pub const fn own(self) -> Own {
        self.own
    }
    pub const fn replay(self) -> Replay {
        self.replay
    }
    pub const fn replay_parent(self) -> Option<u32> {
        self.parent
    }
}
/// One active Map row visited by the authenticated plan (selected domain and
/// required ancestry), with an actually attempted physical WDT request.
/// The dense fixed array is heap storage within the charged Vec allocation.
#[derive(Debug)]
pub struct Map {
    id: u32,
    wdt_file_data_id: u32,
    parent: Option<u32>,
    observed: bool,
    wdt: Wdt,
    global: Option<usize>,
    cells: [u8; 4096],
}
impl Map {
    pub const fn id(&self) -> u32 {
        self.id
    }
    pub const fn wdt_file_data_id(&self) -> u32 {
        self.wdt_file_data_id
    }
    pub const fn parent_map_id(&self) -> Option<u32> {
        self.parent
    }
    pub const fn wdt(&self) -> Wdt {
        self.wdt
    }
    pub const fn global_records(&self) -> Option<usize> {
        self.global
    }
    pub fn tile(&self, x: u8, y: u8) -> Option<Tile> {
        if !self.observed || self.wdt != Wdt::Present || x >= 64 || y >= 64 {
            return None;
        }
        let b = self.cells[usize::from(x) * 64 + usize::from(y)];
        let own = match b & 3 {
            0 => Own::Unlisted,
            1 => Own::Missing,
            2 => Own::Empty,
            _ => Own::Output,
        };
        let replay = match b >> 2 {
            0 => Replay::None,
            1 => Replay::Empty,
            _ => Replay::Output,
        };
        Some(Tile {
            own,
            replay,
            parent: if replay == Replay::None {
                None
            } else {
                self.parent
            },
        })
    }
}
#[derive(Default)]
pub(crate) struct Retained {
    maps: Vec<Map>,
}
impl Retained {
    /// The caller must charge the whole available allocation allowance BEFORE
    /// this allocation, then retain the actual returned capacity charge.
    pub(super) fn allocate(
        maps: &plan::Maps<'_>,
        allowance: usize,
    ) -> Result<Self, super::super::Error> {
        let count = maps.preload().len();
        let need = count
            .checked_mul(size_of::<Map>())
            .ok_or(super::super::Error::Limit)?;
        if need > allowance {
            return Err(super::super::Error::Limit);
        }
        let mut v = Vec::new();
        v.try_reserve_exact(count)
            .map_err(|_| super::super::Error::Allocation)?;
        if v.capacity()
            .checked_mul(size_of::<Map>())
            .is_none_or(|n| n > allowance)
        {
            return Err(super::super::Error::Limit);
        }
        for m in maps.preload() {
            v.push(Map {
                id: m.id(),
                wdt_file_data_id: m.wdt_file_data_id(),
                parent: m.parent_map_id(),
                observed: false,
                wdt: Wdt::Missing,
                global: None,
                cells: [0; 4096],
            });
        }
        v.sort_unstable_by_key(|m| m.id);
        if v.windows(2).any(|p| p[0].id == p[1].id) {
            return Err(super::super::Error::State);
        }
        Ok(Self { maps: v })
    }
    pub(in crate::collision) fn resident_bytes(&self) -> Result<usize, super::super::Error> {
        self.maps
            .capacity()
            .checked_mul(size_of::<Map>())
            .ok_or(super::super::Error::Limit)
    }
    fn map_mut(&mut self, id: u32) -> Result<&mut Map, super::super::Error> {
        let at = self
            .maps
            .binary_search_by_key(&id, |m| m.id)
            .map_err(|_| super::super::Error::State)?;
        Ok(&mut self.maps[at])
    }
    pub(super) fn wdt(&mut self, id: u32, present: bool) -> Result<(), super::super::Error> {
        let m = self.map_mut(id)?;
        if m.observed {
            return Err(super::super::Error::State);
        }
        m.observed = true;
        m.wdt = if present { Wdt::Present } else { Wdt::Missing };
        Ok(())
    }
    pub(super) fn global(&mut self, id: u32, records: usize) -> Result<(), super::super::Error> {
        let m = self.map_mut(id)?;
        if !m.observed || m.wdt != Wdt::Present || m.global.is_some() {
            return Err(super::super::Error::State);
        }
        m.global = Some(records);
        Ok(())
    }
    pub(super) fn own(
        &mut self,
        id: u32,
        x: u8,
        y: u8,
        own: Own,
    ) -> Result<(), super::super::Error> {
        let m = self.map_mut(id)?;
        if m.wdt != Wdt::Present || !m.observed || x >= 64 || y >= 64 || own == Own::Unlisted {
            return Err(super::super::Error::State);
        }
        let b = &mut m.cells[usize::from(x) * 64 + usize::from(y)];
        if *b != 0 {
            return Err(super::super::Error::State);
        }
        *b = match own {
            Own::Missing => 1,
            Own::Empty => 2,
            Own::Output => 3,
            Own::Unlisted => unreachable!(),
        };
        Ok(())
    }
    pub(super) fn replay(
        &mut self,
        id: u32,
        parent: u32,
        x: u8,
        y: u8,
        records: usize,
    ) -> Result<(), super::super::Error> {
        let m = self.map_mut(id)?;
        if !m.observed || m.wdt != Wdt::Present || m.parent != Some(parent) || x >= 64 || y >= 64 {
            return Err(super::super::Error::State);
        }
        let b = &mut m.cells[usize::from(x) * 64 + usize::from(y)];
        if *b > 1 {
            return Err(super::super::Error::State);
        }
        *b |= if records == 0 { 4 } else { 8 };
        Ok(())
    }
    pub(super) fn finish(&self) -> Result<(), super::super::Error> {
        if self
            .maps
            .iter()
            .any(|m| !m.observed || (m.wdt == Wdt::Present) != m.global.is_some())
        {
            return Err(super::super::Error::State);
        }
        Ok(())
    }
    pub(in crate::collision) fn maps(&self) -> &[Map] {
        &self.maps
    }
}
/// Typed callback failure with an independent completed collector guard.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Cause<E> {
    Collector(crate::collision::payload::CollectorError),
    Callback(E),
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Error<E> {
    pub cause: Cause<E>,
    pub guard: Option<crate::collision::payload::CollectorError>,
}
impl<E: Copy> Error<E> {
    pub fn code(self) -> &'static str {
        let source = crate::collision::payload::CollectorError::Collision(
            crate::collision::Error::Store(crate::local_store::Error::InputChanged),
        );
        if self.guard == Some(source) || matches!(self.cause,Cause::Collector(c) if c==source) {
            return source.code();
        }
        if let Some(e) = self.guard {
            return e.code();
        }
        match self.cause {
            Cause::Collector(e) => e.code(),
            Cause::Callback(_) => crate::collision::payload::Error::Callback.code(),
        }
    }
    pub(in crate::collision) fn from_failure(
        e: crate::collision::payload::CallbackFailure<E>,
    ) -> Self {
        match e {
            crate::collision::payload::CallbackFailure::Collector(e) => Self {
                cause: Cause::Collector(e),
                guard: None,
            },
            crate::collision::payload::CallbackFailure::Callback { cause, guard } => Self {
                cause: Cause::Callback(cause),
                guard,
            },
        }
    }
}
impl<E: Copy> std::fmt::Display for Error<E> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.code())
    }
}
impl<E: Copy + std::fmt::Debug> std::error::Error for Error<E> {}
/// Only authentic owners construct this borrowed, allocation-free callback view.
/// Map/GODI available facts and complete liquid facts remain distinct types.
#[derive(Clone, Copy)]
pub struct View<'a> {
    pub(in crate::collision) startup: &'a startup::Data,
    pub(in crate::collision) map: &'a available::TableFacts,
    pub(in crate::collision) liquids: &'a [world::TableFacts; 2],
    pub(in crate::collision) topology: &'a world::Topology,
    pub(in crate::collision) maps: &'a [Map],
    pub(in crate::collision) report: &'a super::Report,
    pub(in crate::collision) profile: &'static str,
}
impl<'a> View<'a> {
    pub const fn startup(self) -> &'a startup::Data {
        self.startup
    }
    pub const fn map_table(self) -> &'a available::TableFacts {
        self.map
    }
    pub const fn liquids(self) -> &'a [world::TableFacts; 2] {
        self.liquids
    }
    pub const fn topology(self) -> &'a world::Topology {
        self.topology
    }
    pub const fn maps(self) -> &'a [Map] {
        self.maps
    }
    pub const fn report(self) -> &'a super::Report {
        self.report
    }
    pub const fn profile(self) -> &'static str {
        self.profile
    }
}
#[cfg(test)]
mod tests;
