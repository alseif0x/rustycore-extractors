// SPDX-License-Identifier: GPL-3.0-or-later
// Authored 2026-10-11: bounded explicit off-mesh connection projection.
//! Preserves source order/duplicates and exact map/row/column selection.
//! No text/global-MMap decoding, endpoint snapping or pathfinding is performed.
//! Finite negative/zero radii and all area/flag bits are retained; Detour input
//! admission is a later stage. Unselected records are not numerically inspected.
use crate::{Error, reserve};

#[derive(Clone, Copy)]
pub struct Source {
    pub map_id: u32,
    pub row: u32,
    pub column: u32,
    pub from: [f32; 3],
    pub to: [f32; 3],
    pub radius: f32,
    pub connection_flags: u8,
    pub area: u8,
    pub flags: u16,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Target {
    pub map_id: u32,
    pub row: u8,
    pub column: u8,
}
#[derive(Clone, Copy)]
pub struct Connection {
    endpoints: [[f32; 3]; 2],
    radius: f32,
    direction: u8,
    area: u8,
    flags: u16,
}
impl Connection {
    pub const fn endpoints(&self) -> [[f32; 3]; 2] {
        self.endpoints
    }
    pub const fn radius(&self) -> f32 {
        self.radius
    }
    pub const fn direction(&self) -> u8 {
        self.direction
    }
    pub const fn area(&self) -> u8 {
        self.area
    }
    pub const fn flags(&self) -> u16 {
        self.flags
    }
}
pub struct Connections {
    values: Vec<Connection>,
}
impl Connections {
    pub fn values(&self) -> &[Connection] {
        &self.values
    }
    pub fn capacity_bytes(&self) -> usize {
        self.values.capacity() * std::mem::size_of::<Connection>()
    }
}
#[derive(Clone, Copy, Debug)]
pub struct Limits {
    pub input_records: usize,
    pub output_records: usize,
    pub heap_bytes: usize,
}
impl Default for Limits {
    fn default() -> Self {
        Self {
            input_records: 65536,
            output_records: 65536,
            heap_bytes: 2 * 1024 * 1024,
        }
    }
}
pub fn select(target: Target, source: &[Source], limits: Limits) -> Result<Connections, Error> {
    if target.row >= 64 || target.column >= 64 {
        return Err(Error::Coordinates);
    }
    if limits.input_records > 65536
        || limits.output_records > 65536
        || limits.heap_bytes > 2 * 1024 * 1024
        || source.len() > limits.input_records
    {
        return Err(Error::Limit);
    }
    let selected = || {
        source.iter().filter(|s| {
            s.map_id == target.map_id
                && s.row == u32::from(target.row)
                && s.column == u32::from(target.column)
        })
    };
    let count = selected().count();
    if count > limits.output_records
        || count * std::mem::size_of::<Connection>() > limits.heap_bytes
    {
        return Err(Error::Limit);
    }
    // Check all selected records before allocating output.
    if selected().any(|s| {
        !s.from
            .iter()
            .chain(&s.to)
            .chain(std::iter::once(&s.radius))
            .all(|v| v.is_finite())
    }) {
        return Err(Error::NonFinite);
    }
    let mut values = Vec::new();
    reserve(&mut values, count)?;
    if values.capacity() * std::mem::size_of::<Connection>() > limits.heap_bytes {
        return Err(Error::Limit);
    }
    for s in selected() {
        values.push(Connection {
            endpoints: [
                [s.from[1], s.from[2], s.from[0]],
                [s.to[1], s.to[2], s.to[0]],
            ],
            radius: s.radius,
            direction: s.connection_flags & 1,
            area: s.area,
            flags: s.flags,
        });
    }
    Ok(Connections { values })
}

#[cfg(test)]
mod tests;
