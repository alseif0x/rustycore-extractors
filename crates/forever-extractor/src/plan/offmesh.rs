// SPDX-License-Identifier: GPL-3.0-or-later
//! Explicit inline connection bits; no source discovery or coordinate rewriting.
use super::decimal;
use forever_navigation::offmesh;
use std::fmt::Write;

pub const OFFMESH_PROFILE: &str = "inline-f32-bits-v1";
pub const MAX_OFFMESH_CONNECTIONS: usize = 1024;

/// Original source axes and float bits. Equality retains signed zero, order and
/// duplicates; conversion to navigation sources changes no float bits.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct OffMeshConnection {
    pub map_id: u32,
    pub row: u8,
    pub column: u8,
    pub from_bits: [u32; 3],
    pub to_bits: [u32; 3],
    pub radius_bits: u32,
    pub direction: u8,
    pub area: u8,
    pub flags: u16,
}

impl OffMeshConnection {
    pub fn validate(&self) -> Result<(), &'static str> {
        if self.row >= 64
            || self.column >= 64
            || self.direction > 1
            || self.area > 63
            || self
                .from_bits
                .iter()
                .chain(&self.to_bits)
                .chain(std::iter::once(&self.radius_bits))
                .any(|bits| !f32::from_bits(*bits).is_finite())
            || f32::from_bits(self.radius_bits) <= 0.0
        {
            return Err("product_plan_offmesh");
        }
        Ok(())
    }

    pub fn source(self) -> offmesh::Source {
        offmesh::Source {
            map_id: self.map_id,
            row: u32::from(self.row),
            column: u32::from(self.column),
            from: self.from_bits.map(f32::from_bits),
            to: self.to_bits.map(f32::from_bits),
            radius: f32::from_bits(self.radius_bits),
            connection_flags: self.direction,
            area: self.area,
            flags: self.flags,
        }
    }

    /// Reject source coordinates that cannot be represented by this profile.
    pub fn from_source(source: offmesh::Source) -> Result<Self, &'static str> {
        let result = Self {
            map_id: source.map_id,
            row: source.row.try_into().map_err(|_| "product_plan_offmesh")?,
            column: source
                .column
                .try_into()
                .map_err(|_| "product_plan_offmesh")?,
            from_bits: source.from.map(f32::to_bits),
            to_bits: source.to.map(f32::to_bits),
            radius_bits: source.radius.to_bits(),
            direction: source.connection_flags,
            area: source.area,
            flags: source.flags,
        };
        result.validate()?;
        Ok(result)
    }

    pub(super) fn parse(line: &str) -> Result<Self, &'static str> {
        let fields: Vec<_> = line.split('\t').collect();
        let [
            "offmesh-connection",
            map,
            row,
            column,
            fx,
            fy,
            fz,
            tx,
            ty,
            tz,
            radius,
            direction,
            area,
            flags,
        ] = fields.as_slice()
        else {
            return Err("product_plan_format");
        };
        let result = Self {
            map_id: decimal(map)?,
            row: decimal(row)?
                .try_into()
                .map_err(|_| "product_plan_offmesh")?,
            column: decimal(column)?
                .try_into()
                .map_err(|_| "product_plan_offmesh")?,
            from_bits: [bits(fx)?, bits(fy)?, bits(fz)?],
            to_bits: [bits(tx)?, bits(ty)?, bits(tz)?],
            radius_bits: bits(radius)?,
            direction: decimal(direction)?
                .try_into()
                .map_err(|_| "product_plan_offmesh")?,
            area: decimal(area)?
                .try_into()
                .map_err(|_| "product_plan_offmesh")?,
            flags: decimal(flags)?
                .try_into()
                .map_err(|_| "product_plan_offmesh")?,
        };
        result.validate()?;
        Ok(result)
    }

    pub(super) fn render(self, output: &mut String) {
        writeln!(
            output,
            "offmesh-connection\t{}\t{}\t{}\t{:08x}\t{:08x}\t{:08x}\t{:08x}\t{:08x}\t{:08x}\t{:08x}\t{}\t{}\t{}",
            self.map_id, self.row, self.column,
            self.from_bits[0], self.from_bits[1], self.from_bits[2],
            self.to_bits[0], self.to_bits[1], self.to_bits[2], self.radius_bits,
            self.direction, self.area, self.flags,
        )
        .expect("writing into String cannot fail");
    }
}

fn bits(value: &str) -> Result<u32, &'static str> {
    if value.len() != 8
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    {
        return Err("product_plan_offmesh_bits");
    }
    u32::from_str_radix(value, 16).map_err(|_| "product_plan_offmesh_bits")
}
