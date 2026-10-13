// SPDX-License-Identifier: GPL-3.0-or-later
// Authored 2026-10-11: original single-doodad projection from pinned operation
// facts, not imported G3D implementation. Pin 02245dcd245e7433e524577656177723d3e4992e:
// model.cpp:217-275; Matrix3.cpp:122-141,278-286,1383-1408,1664-1680;
// Matrix3.h:170-177; Quat.h:79,150-167,347-348; Quat.cpp:148-160;
// g3dmath.h:148-158,381-382,647-665,807-812; Vector3.h:656-657,685-688.
//! Projects one selected WMO doodad. The caller supplies the assigned ID and
//! normalized name; selection, source unordered-set iteration, native ID
//! allocation and placement emission order remain outside this function.
//!
//! Arithmetic preserves f32 operation order and quaternion normalization.
//! Parent sin/cos and normalization sqrt use f32 operations; Euler asin/atan2
//! use f64 then round to f32. Their standard-library precision is platform
//! dependent. Exact native parity requires the recorded Linux/toolchain
//! differential; no portable libm bit-equality promise is made. No fused
//! multiply-add or approximate transcendental implementation is used here.

use super::{Context, Error, Limits, Modf, Placement, finite, permute, valid_name};
use crate::wmo::DoodadSpawn;

#[cfg(test)]
mod tests;

type Matrix = [[f32; 3]; 3];

// The pinned double literals pi() = 3.1415926535898 and halfPi() =
// 1.57079633 round to these f32 words at their call sites.
const PI: f32 = f32::from_bits(0x4049_0fdb);
const HALF_PI: f32 = f32::from_bits(0x3fc9_0fdb);

/// Single `Doodad::ExtractSet` projection, after successful dependency selection.
/// Source quaternion values are normalized by the original constructor path;
/// they need not already be unit length. A zero/underflowed squared norm would
/// produce nonfinite output and rejects. Overflow of a finite squared norm is
/// retained: reciprocal sqrt becomes zero and its normalized quaternion yields
/// the source's finite identity result. Only finite source/emitted values are
/// admitted. The parent MODF's scale, path flag, NameSet and bounds do not alter
/// the child. Its already-validated bounds remain a source admission policy.
pub fn project<'a>(
    parent: &Modf<'_>,
    spawn: &DoodadSpawn,
    assigned_id: u32,
    name: &'a [u8],
    context: Context,
    limits: Limits,
) -> Result<Placement<'a>, Error> {
    limits.one(38usize.checked_add(name.len()).ok_or(Error::Limit)?)?;
    valid_name(name, limits)?;
    // DoodadSpawn has public fields and may be constructed without Root::parse.
    finite(&spawn.position_bits)?;
    finite(&spawn.rotation_bits)?;
    finite(&[spawn.scale_bits])?;

    let mut origin = permute(parent.position_bits()).map(f32::from_bits);
    if context.global_wmo {
        // Exact pinned f32 decimal-product result. Include the third addition:
        // unlike copying the original word, -0.0 + +0.0 produces +0.0.
        let offsets = [
            f32::from_bits(0x4685_5555),
            f32::from_bits(0x4685_5555),
            0.0,
        ];
        for (value, offset) in origin.iter_mut().zip(offsets) {
            *value += offset;
        }
    }
    let angles = parent.rotation_bits().map(f32::from_bits);
    let parent_rotation = parent_rotation(angles);
    let transformed = transform(&parent_rotation, spawn.position_bits.map(f32::from_bits));
    let position = std::array::from_fn(|i| (origin[i] + transformed[i]).to_bits());

    let child_rotation = quaternion_matrix(spawn.rotation_bits.map(f32::from_bits))?;
    let combined = multiply(&child_rotation, &parent_rotation);
    let [x, y, z] = euler_xyz(&combined);
    // Native references are (rotation.z, rotation.x, rotation.y), followed by
    // separate float degree conversions. Parent scale never participates.
    let rotation = [
        degrees(y).to_bits(),
        degrees(z).to_bits(),
        degrees(x).to_bits(),
    ];
    let result = Placement {
        flags: if context.map_id != context.original_map_id {
            2
        } else {
            0
        },
        name_set: 0,
        id: assigned_id,
        position,
        rotation,
        scale: spawn.scale_bits,
        bounds: None,
        name,
    };
    result.validate(limits)?;
    Ok(result)
}

fn radians(value: f32) -> f32 {
    (value * PI) / 180.0_f32
}

fn degrees(value: f32) -> f32 {
    (value * 180.0_f32) / PI
}

fn parent_rotation(rotation: [f32; 3]) -> Matrix {
    let z = radians(rotation[1]);
    let y = radians(rotation[0]);
    let x = radians(rotation[2]);
    // Separate sin/cos calls match the original float overloads. Do not replace
    // with sin_cos, change argument precision or algebraically collapse Z*Y*X.
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

fn transform(matrix: &Matrix, vector: [f32; 3]) -> [f32; 3] {
    matrix.map(|row| dot(row, vector))
}

fn quaternion_matrix(q: [f32; 4]) -> Result<Matrix, Error> {
    let norm = (((q[0] * q[0]) + (q[1] * q[1])) + (q[2] * q[2])) + (q[3] * q[3]);
    if norm == 0.0 {
        return Err(Error::NonFinite);
    }
    // Do not reject +infinity here: finite components with an overflowing
    // squared norm become signed zeros, as in the pinned normalization.
    let inverse = 1.0_f32 / norm.sqrt();
    let [x, y, z, w] = q.map(|value| value * inverse);
    let xx = (2.0_f32 * x) * x;
    let xy = (2.0_f32 * x) * y;
    let xz = (2.0_f32 * x) * z;
    let xw = (2.0_f32 * x) * w;
    let yy = (2.0_f32 * y) * y;
    let yz = (2.0_f32 * y) * z;
    let yw = (2.0_f32 * y) * w;
    let zz = (2.0_f32 * z) * z;
    let zw = (2.0_f32 * z) * w;
    Ok([
        [(1.0_f32 - yy) - zz, xy - zw, xz + yw],
        [xy + zw, (1.0_f32 - xx) - zz, yz - xw],
        [xz - yw, yz + xw, (1.0_f32 - xx) - yy],
    ])
}

fn euler_xyz(matrix: &Matrix) -> [f32; 3] {
    let value = matrix[0][2];
    if value < 1.0 {
        if value > -1.0 {
            [
                f64::from(-matrix[1][2]).atan2(f64::from(matrix[2][2])) as f32,
                f64::from(value).asin() as f32,
                f64::from(-matrix[0][1]).atan2(f64::from(matrix[0][0])) as f32,
            ]
        } else {
            [
                -(f64::from(matrix[1][0]).atan2(f64::from(matrix[1][1])) as f32),
                -HALF_PI,
                0.0,
            ]
        }
    } else {
        [
            f64::from(matrix[1][0]).atan2(f64::from(matrix[1][1])) as f32,
            HALF_PI,
            0.0,
        ]
    }
}
