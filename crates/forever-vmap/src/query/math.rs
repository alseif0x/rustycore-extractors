// SPDX-License-Identifier: GPL-3.0-or-later
// Authored 2026-10-11: original scalar operations from pinned G3D facts.
use super::Ray;

// std::min/max preserve their FIRST operand on equality or unordered operands.
// Rust float min/max have different NaN handling; do not substitute them.
pub(crate) fn first_min(a: f32, b: f32) -> f32 {
    if b < a { b } else { a }
}
pub(crate) fn first_max(a: f32, b: f32) -> f32 {
    if a < b { b } else { a }
}

// fuzzyNe only has the double overload in the pinned g3dmath.h862. The float
// direction is promoted; using fuzzyEq(float) or its 1e-5 epsilon is incorrect.
pub(crate) fn fuzzy_nonzero(value: f32) -> bool {
    let value = f64::from(value);
    !(value == 0.0 || value.abs() <= 0.000_000_5 * (value.abs() + 1.0))
}

/// Exact candidate-plane Ray::intersectionTime(AABox) operation shape.
/// Bounds and ray have already been validated. The native helper can produce
/// NaN in degenerate signed-zero cases; instance callers compare only +infinity.
/// This is deliberately not a generic slab test or parameter-distance result.
pub(crate) fn box_intersection(ray: &Ray, bounds: [f32; 6]) -> f32 {
    let origin = ray.origin();
    let direction = ray.direction();
    let mut location = [0.0; 3];
    let mut candidate = [-1.0f32; 3];
    let mut inside = true;
    for axis in 0..3 {
        if origin[axis] < bounds[axis] {
            location[axis] = bounds[axis];
            inside = false;
            if direction[axis].to_bits() != 0 {
                candidate[axis] = (bounds[axis] - origin[axis]) / direction[axis];
            }
        } else if origin[axis] > bounds[axis + 3] {
            location[axis] = bounds[axis + 3];
            inside = false;
            if direction[axis].to_bits() != 0 {
                candidate[axis] = (bounds[axis + 3] - origin[axis]) / direction[axis];
            }
        }
    }
    if inside {
        return 0.0;
    }
    let mut plane = 0;
    if candidate[1] > candidate[plane] {
        plane = 1;
    }
    if candidate[2] > candidate[plane] {
        plane = 2;
    }
    if candidate[plane].is_sign_negative() {
        return f32::INFINITY;
    }
    for axis in 0..3 {
        if axis != plane {
            location[axis] = origin[axis] + candidate[plane] * direction[axis];
            if location[axis] < bounds[axis] || location[axis] > bounds[axis + 3] {
                return f32::INFINITY;
            }
        }
    }
    let d = [
        location[0] - origin[0],
        location[1] - origin[1],
        location[2] - origin[2],
    ];
    ((d[0] * d[0] + d[1] * d[1]) + d[2] * d[2]).sqrt()
}
