// SPDX-License-Identifier: GPL-3.0-or-later
// Authored 2026-10-11: original Rust from pinned transformed-bound operation
// facts. No TrinityCore/G3D implementation is copied or imported.
//! Complete a placement's missing bounds from the supplied RAW group zero.
//!
//! Pin 02245dcd245e7433e524577656177723d3e4992e: TileAssembler.cpp
//! 46-51/177-193/335-357, TileAssembler.h48-54, AABox.cpp361-414,
//! AABox.h106-123, Matrix3.cpp278-290/1664-1680 and Vector3.h778-784.
//! Apply scale, then Z*(Y*X), merge eight corners in the source order, then
//! add position. New points win equal signed-zero extrema. Source/model
//! acquisition, group selection, class checking, runtime queries and I/O are
//! outside this API. Existing bounds need no model and remain unchanged.
//!
//! Inputs, transformed corners and final bounds must be finite and ordered.
//! Negative/zero scales are admitted. Floating operations retain source order;
//! sin/cos use f32 as in the reviewed Linux C++ toolchain. No FMA, approximate
//! trigonometry or portable cross-libm bit-equality claim is made. No heap is
//! allocated and no input or supplied placement is mutated.

use super::{Error as PlacementError, Limits, Placement, finite, valid_bounds};

#[cfg(test)]
mod tests;

/// Completion requires a model only when the placement has no supplied bounds.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Error {
    MissingModel,
    Placement(PlacementError),
}
impl Error {
    pub const fn code(self) -> &'static str {
        match self {
            Self::MissingModel => "placement_bounds_missing_model",
            Self::Placement(error) => error.code(),
        }
    }
}
impl From<PlacementError> for Error {
    fn from(value: PlacementError) -> Self {
        Self::Placement(value)
    }
}
impl std::fmt::Display for Error {
    fn fmt(&self, out: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        out.write_str(self.code())
    }
}
impl std::error::Error for Error {}

const PI: f32 = f32::from_bits(0x4049_0fdb);
type Matrix = [[f32; 3]; 3];
// LLH, HLH, HHH, LHH, LLL, HLL, HHL, LHL (low XYZ then high XYZ).
const CORNERS: [[usize; 3]; 8] = [
    [0, 1, 5],
    [3, 1, 5],
    [3, 4, 5],
    [0, 4, 5],
    [0, 1, 2],
    [3, 1, 2],
    [3, 4, 2],
    [0, 4, 2],
];

/// Project the exact eight corners of one complete supplied model bound.
/// Rotation components have the placement's native order, with Z/Y/X matrix
/// angles sourced from rotation.y / rotation.x / rotation.z respectively.
pub fn transform(
    raw_group0_bounds: [u32; 6],
    position: [u32; 3],
    rotation: [u32; 3],
    scale: u32,
) -> Result<[u32; 6], PlacementError> {
    valid_bounds(&raw_group0_bounds)?;
    finite(&position)?;
    finite(&rotation)?;
    finite(&[scale])?;
    let matrix = rotation_matrix(rotation.map(f32::from_bits));
    let source = raw_group0_bounds.map(f32::from_bits);
    let scale = f32::from_bits(scale);
    let mut low = [0.0; 3];
    let mut high = [0.0; 3];
    for (ordinal, corner) in CORNERS.into_iter().enumerate() {
        let scaled = corner.map(|index| source[index] * scale);
        let point = matrix.map(|row| dot(row, scaled));
        finite(&point.map(f32::to_bits))?;
        if ordinal == 0 {
            low = point;
            high = point;
        } else {
            for axis in 0..3 {
                if point[axis] <= low[axis] {
                    low[axis] = point[axis];
                }
                if point[axis] >= high[axis] {
                    high[axis] = point[axis];
                }
            }
        }
    }
    let position = position.map(f32::from_bits);
    let output = std::array::from_fn(|index| {
        let coordinate = if index < 3 {
            low[index]
        } else {
            high[index - 3]
        };
        (coordinate + position[index % 3]).to_bits()
    });
    valid_bounds(&output)?;
    Ok(output)
}

/// Revalidate the placement under the supplied downward limits. If already
/// bounded, return it unchanged without inspecting `model_bounds` (even None).
/// Otherwise require model bounds, add the transformed bounds and flag bit 1,
/// then revalidate the enlarged wire length before returning the new value.
pub fn complete<'a>(
    mut placement: Placement<'a>,
    model_bounds: Option<[u32; 6]>,
    limits: Limits,
) -> Result<Placement<'a>, Error> {
    placement.validate(limits)?;
    if placement.bounds.is_some() {
        return Ok(placement);
    }
    let model_bounds = model_bounds.ok_or(Error::MissingModel)?;
    placement.bounds = Some(transform(
        model_bounds,
        placement.position,
        placement.rotation,
        placement.scale,
    )?);
    placement.flags |= 1;
    placement.validate(limits)?;
    Ok(placement)
}

fn rotation_matrix(rotation: [f32; 3]) -> Matrix {
    // Pinned ModelPosition::init uses pif()*component/180.f, with the constant
    // on the LEFT. Do not replace with to_radians or a precomputed factor.
    let z = (PI * rotation[1]) / 180.0_f32;
    let y = (PI * rotation[0]) / 180.0_f32;
    let x = (PI * rotation[2]) / 180.0_f32;
    let (cz, sz) = (z.cos(), z.sin());
    let (cy, sy) = (y.cos(), y.sin());
    let (cx, sx) = (x.cos(), x.sin());
    let rz = [[cz, -sz, 0.0], [sz, cz, 0.0], [0.0, 0.0, 1.0]];
    let ry = [[cy, 0.0, sy], [0.0, 1.0, 0.0], [-sy, 0.0, cy]];
    let rx = [[1.0, 0.0, 0.0], [0.0, cx, -sx], [0.0, sx, cx]];
    multiply(&rz, &multiply(&ry, &rx))
}
fn dot(a: [f32; 3], b: [f32; 3]) -> f32 {
    ((a[0] * b[0]) + (a[1] * b[1])) + (a[2] * b[2])
}
fn multiply(a: &Matrix, b: &Matrix) -> Matrix {
    std::array::from_fn(|row| {
        std::array::from_fn(|column| dot(a[row], [b[0][column], b[1][column], b[2][column]]))
    })
}
