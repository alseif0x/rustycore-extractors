// SPDX-License-Identifier: GPL-3.0-or-later
// Authored 2026-10-11: original bounded model/instance ray-query implementation.
//! Queries borrow a completely validated VMO view or immutable owned model.
//! Authored modification 2026-10-11: private geometry adapters retain the same
//! traversal and math for both storage forms, without per-query allocations.
//! Reference pin
//! 02245dcd245e7433e524577656177723d3e4992e, WorldModel.cpp34-81/390-505,
//! ModelInstance.cpp27-65, G3D Matrix3.cpp278-290/404-450/1664-1680.
//! Triangle and group BIHs use the shared native-order walker and work budget.
//! A single group bypasses its declared bound/group BIH; GroupModel itself only
//! traverses its mesh BIH. The two-sided triangle test retains strict determinant
//! and distance comparisons and inclusive barycentric edges. Rays are NEVER
//! normalized and supplied maximum distance is a ray parameter, not necessarily
//! Euclidean length. Any stops in traversal order, not at the closest triangle.
//!
//! Instance queries preserve the general cofactor inverse and f32 operation
//! order; a transpose shortcut is not equivalent. Only finite nonzero scales
//! with finite reciprocals are admitted. Zero/singular/nonfinite-transform cases
//! return errors rather than native undefined or misleading numeric results.
//! The broadphase rejects only +infinity from the original AABox algorithm;
//! it does not compare box time with maximum distance. No area/liquid/LOS/height
//! convenience, model acquisition, placement-name binding or stateful runtime is
//! implied. No heap allocations or sensitive Debug representation are provided.
use super::{Budget, Error, Mode, Ray, math, traverse, validate_distance};
use crate::{bih, placement::Placement, vmo};

#[cfg(test)]
mod tests;

type Matrix = [[f32; 3]; 3];
const PI: f32 = f32::from_bits(0x4049_0fdb);

// Private, closed adapters: callers cannot forge a geometry implementation.
// Group and BIH values contain only borrowed slices and Copy metadata.
trait Geometry {
    fn flags(&self) -> u32;
    fn group_count(&self) -> usize;
    fn group(&self, index: usize) -> Option<vmo::Group<'_>>;
    fn group_bih(&self) -> Option<bih::View<'_>>;
}
impl Geometry for vmo::View<'_> {
    fn flags(&self) -> u32 {
        self.flags()
    }
    fn group_count(&self) -> usize {
        self.groups().len()
    }
    fn group(&self, index: usize) -> Option<vmo::Group<'_>> {
        self.groups().get(index).copied()
    }
    fn group_bih(&self) -> Option<bih::View<'_>> {
        self.group_bih().copied()
    }
}
impl Geometry for vmo::Owned {
    fn flags(&self) -> u32 {
        self.flags()
    }
    fn group_count(&self) -> usize {
        self.group_count()
    }
    fn group(&self, index: usize) -> Option<vmo::Group<'_>> {
        self.group(index)
    }
    fn group_bih(&self) -> Option<bih::View<'_>> {
        self.group_bih()
    }
}

#[derive(Clone, Copy)]
pub struct Hit {
    distance: f32,
    group: u32,
    triangle: u32,
}
impl Hit {
    pub const fn distance(self) -> f32 {
        self.distance
    }
    pub const fn distance_bits(self) -> u32 {
        self.distance.to_bits()
    }
    pub const fn group_index(self) -> u32 {
        self.group
    }
    pub const fn triangle_index(self) -> u32 {
        self.triangle
    }
}
fn scalar(value: f32) -> Result<f32, Error> {
    if value.is_finite() {
        Ok(value)
    } else {
        Err(Error::NonFiniteResult)
    }
}
fn finite<const N: usize>(values: [f32; N]) -> Result<[f32; N], Error> {
    for value in values {
        scalar(value)?;
    }
    Ok(values)
}
fn dot(a: [f32; 3], b: [f32; 3]) -> f32 {
    ((a[0] * b[0]) + (a[1] * b[1])) + (a[2] * b[2])
}
fn subtract(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    std::array::from_fn(|i| a[i] - b[i])
}
fn cross(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}

/// Query a model in its own coordinate space. Input/model validation errors
/// and work-budget exhaustion remain distinct from `Ok(None)` (a miss).
pub fn intersect(
    model: &vmo::View<'_>,
    ray: &Ray,
    max_distance: f32,
    mode: Mode,
    ignore_m2: bool,
    budget: &mut Budget,
) -> Result<Option<Hit>, Error> {
    validate_distance(max_distance)?;
    intersect_inner(model, ray, max_distance, mode, ignore_m2, budget)
}
/// Same checked query and work accounting as `intersect`, reusing a model
/// validated at load time. No complete parsing or allocation occurs here.
pub fn intersect_owned(
    model: &vmo::Owned,
    ray: &Ray,
    max_distance: f32,
    mode: Mode,
    ignore_m2: bool,
    budget: &mut Budget,
) -> Result<Option<Hit>, Error> {
    validate_distance(max_distance)?;
    intersect_inner(model, ray, max_distance, mode, ignore_m2, budget)
}
// The original instance path can supply negative distance after a negative
// scale. Do not apply the public nonnegative-distance admission a second time.
fn intersect_inner(
    model: &impl Geometry,
    ray: &Ray,
    max_distance: f32,
    mode: Mode,
    ignore_m2: bool,
    budget: &mut Budget,
) -> Result<Option<Hit>, Error> {
    if ignore_m2 && model.flags() & 1 != 0 {
        return Ok(None);
    }
    let mut distance = max_distance;
    let mut hit = None;
    if model.group_count() == 1 {
        intersect_group(
            &model.group(0).ok_or(Error::InvalidModel)?,
            0,
            ray,
            &mut distance,
            mode,
            budget,
            &mut hit,
        )?;
    } else if let Some(tree) = model.group_bih() {
        traverse::ray(
            &tree,
            ray,
            &mut distance,
            mode,
            budget,
            |ordinal, distance, budget| {
                let group = model.group(ordinal as usize).ok_or(Error::InvalidModel)?;
                intersect_group(&group, ordinal, ray, distance, mode, budget, &mut hit)
            },
        )?;
    } else if model.group_count() != 0 {
        return Err(Error::InvalidModel);
    }
    Ok(hit)
}
fn intersect_group(
    group: &vmo::Group<'_>,
    ordinal: u32,
    ray: &Ray,
    distance: &mut f32,
    mode: Mode,
    budget: &mut Budget,
    hit: &mut Option<Hit>,
) -> Result<bool, Error> {
    if group.triangle_count() == 0 {
        return Ok(false);
    }
    let tree = group.mesh_bih().ok_or(Error::InvalidModel)?;
    let mut group_hit = false;
    traverse::ray(
        tree,
        ray,
        distance,
        mode,
        budget,
        |triangle, distance, budget| {
            budget.charge_triangle()?;
            let indices = group
                .triangle(triangle as usize)
                .ok_or(Error::InvalidModel)?;
            let mut points = [[0.0; 3]; 3];
            for (i, index) in indices.into_iter().enumerate() {
                points[i] = group
                    .vertex_bits(index as usize)
                    .ok_or(Error::InvalidModel)?
                    .map(f32::from_bits);
            }
            if let Some(new_distance) = intersect_triangle(points, ray, *distance)? {
                *distance = new_distance;
                *hit = Some(Hit {
                    distance: new_distance,
                    group: ordinal,
                    triangle,
                });
                group_hit = true;
            }
            // Native callback retains hit with |= even on later misses.
            Ok(group_hit)
        },
    )?;
    Ok(group_hit)
}
fn intersect_triangle(
    points: [[f32; 3]; 3],
    ray: &Ray,
    distance: f32,
) -> Result<Option<f32>, Error> {
    let e1 = finite(subtract(points[1], points[0]))?;
    let e2 = finite(subtract(points[2], points[0]))?;
    let p = finite(cross(ray.direction(), e2))?;
    let determinant = scalar(dot(e1, p))?;
    if determinant.abs() < 1e-5_f32 {
        return Ok(None);
    }
    let factor = scalar(1.0_f32 / determinant)?;
    let s = finite(subtract(ray.origin(), points[0]))?;
    let u = scalar(factor * scalar(dot(s, p))?)?;
    if !(0.0..=1.0).contains(&u) {
        return Ok(None);
    }
    let q = finite(cross(s, e1))?;
    let v = scalar(factor * scalar(dot(ray.direction(), q))?)?;
    if v < 0.0 || scalar(u + v)? > 1.0 {
        return Ok(None);
    }
    let t = scalar(factor * scalar(dot(e2, q))?)?;
    Ok((t > 0.0 && t < distance).then_some(t))
}

/// Precomputed, allocation-free instance transform. Model identity/name binding
/// remains the caller's obligation; the QA envelope supplies exactly one model.
pub struct Instance {
    position: [f32; 3],
    bounds: [f32; 6],
    inverse: Matrix,
    inverse_scale: f32,
    scale: f32,
}
impl Instance {
    /// Checked stored transform components for consumers such as navigation.
    /// The inverse is the original cofactor inverse, not a rotation transpose.
    /// Returning copies cannot change the instance's cached query state.
    pub const fn position(&self) -> [f32; 3] {
        self.position
    }
    pub const fn inverse_rotation(&self) -> [[f32; 3]; 3] {
        self.inverse
    }
    pub const fn scale(&self) -> f32 {
        self.scale
    }
    pub fn new(spawn: &Placement<'_>) -> Result<Self, Error> {
        let bounds = spawn
            .bounds_bits()
            .ok_or(Error::MissingBounds)?
            .map(f32::from_bits);
        let position = spawn.position_bits().map(f32::from_bits);
        let scale = f32::from_bits(spawn.scale_bits());
        if !scale.is_finite() || scale == 0.0 {
            return Err(Error::InvalidScale);
        }
        let inverse_scale = 1.0_f32 / scale;
        if !inverse_scale.is_finite() {
            return Err(Error::InvalidScale);
        }
        let rotation = rotation(spawn.rotation_bits().map(f32::from_bits))?;
        let inverse = invert(rotation)?;
        Ok(Self {
            position,
            bounds,
            inverse,
            inverse_scale,
            scale,
        })
    }
    pub fn intersect(
        &self,
        model: &vmo::View<'_>,
        ray: &Ray,
        max_distance: f32,
        mode: Mode,
        ignore_m2: bool,
        budget: &mut Budget,
    ) -> Result<Option<Hit>, Error> {
        self.intersect_inner(model, ray, max_distance, mode, ignore_m2, budget)
    }
    /// Reuses owned model descriptors and this precomputed transform. Numeric
    /// operations and budget charging are shared with the borrowed-view path.
    pub fn intersect_owned(
        &self,
        model: &vmo::Owned,
        ray: &Ray,
        max_distance: f32,
        mode: Mode,
        ignore_m2: bool,
        budget: &mut Budget,
    ) -> Result<Option<Hit>, Error> {
        self.intersect_inner(model, ray, max_distance, mode, ignore_m2, budget)
    }
    fn intersect_inner(
        &self,
        model: &impl Geometry,
        ray: &Ray,
        max_distance: f32,
        mode: Mode,
        ignore_m2: bool,
        budget: &mut Budget,
    ) -> Result<Option<Hit>, Error> {
        validate_distance(max_distance)?;
        if math::box_intersection(ray, self.bounds) == f32::INFINITY {
            return Ok(None);
        }
        let origin = finite(subtract(ray.origin(), self.position))?;
        let origin = finite(multiply_vector(self.inverse, origin))?;
        let origin = finite(origin.map(|value| value * self.inverse_scale))?;
        let direction = finite(multiply_vector(self.inverse, ray.direction()))?;
        let transformed = Ray::new(origin, direction)?;
        let max_distance = max_distance * self.inverse_scale;
        if max_distance.is_nan() {
            return Err(Error::NonFiniteResult);
        }
        let Some(mut hit) =
            intersect_inner(model, &transformed, max_distance, mode, ignore_m2, budget)?
        else {
            return Ok(None);
        };
        hit.distance *= self.scale;
        if hit.distance.is_nan() {
            return Err(Error::NonFiniteResult);
        }
        Ok(Some(hit))
    }
}
/// One-shot convenience; repeated queries should retain `Instance` to reuse the
/// same checked cofactor inverse without changing operations or model results.
pub fn instance(
    model: &vmo::View<'_>,
    spawn: &Placement<'_>,
    ray: &Ray,
    max_distance: f32,
    mode: Mode,
    ignore_m2: bool,
    budget: &mut Budget,
) -> Result<Option<Hit>, Error> {
    Instance::new(spawn)?.intersect(model, ray, max_distance, mode, ignore_m2, budget)
}
/// One-shot owned-model convenience. Retain `Instance` for repeated queries.
pub fn instance_owned(
    model: &vmo::Owned,
    spawn: &Placement<'_>,
    ray: &Ray,
    max_distance: f32,
    mode: Mode,
    ignore_m2: bool,
    budget: &mut Budget,
) -> Result<Option<Hit>, Error> {
    Instance::new(spawn)?.intersect_owned(model, ray, max_distance, mode, ignore_m2, budget)
}
fn multiply_vector(matrix: Matrix, vector: [f32; 3]) -> [f32; 3] {
    matrix.map(|row| dot(row, vector))
}
fn multiply(a: Matrix, b: Matrix) -> Matrix {
    std::array::from_fn(|row| {
        std::array::from_fn(|col| dot(a[row], [b[0][col], b[1][col], b[2][col]]))
    })
}
fn rotation(rotation: [f32; 3]) -> Result<Matrix, Error> {
    let [z, y, x] = finite([
        (PI * rotation[1]) / 180.0_f32,
        (PI * rotation[0]) / 180.0_f32,
        (PI * rotation[2]) / 180.0_f32,
    ])?;
    let (cz, sz) = (z.cos(), z.sin());
    let (cy, sy) = (y.cos(), y.sin());
    let (cx, sx) = (x.cos(), x.sin());
    let rz = [[cz, -sz, 0.0], [sz, cz, 0.0], [0.0, 0.0, 1.0]];
    let ry = [[cy, 0.0, sy], [0.0, 1.0, 0.0], [-sy, 0.0, cy]];
    let rx = [[1.0, 0.0, 0.0], [0.0, cx, -sx], [0.0, sx, cx]];
    let out = multiply(rz, multiply(ry, rx));
    for row in out {
        finite(row)?;
    }
    Ok(out)
}
fn invert(m: Matrix) -> Result<Matrix, Error> {
    // General inverse as used by the original Matrix3, NOT rotation transpose.
    let mut out = [
        [
            m[1][1] * m[2][2] - m[1][2] * m[2][1],
            m[0][2] * m[2][1] - m[0][1] * m[2][2],
            m[0][1] * m[1][2] - m[0][2] * m[1][1],
        ],
        [
            m[1][2] * m[2][0] - m[1][0] * m[2][2],
            m[0][0] * m[2][2] - m[0][2] * m[2][0],
            m[0][2] * m[1][0] - m[0][0] * m[1][2],
        ],
        [
            m[1][0] * m[2][1] - m[1][1] * m[2][0],
            m[0][1] * m[2][0] - m[0][0] * m[2][1],
            m[0][0] * m[1][1] - m[0][1] * m[1][0],
        ],
    ];
    for row in out {
        finite(row)?;
    }
    let determinant = scalar(dot(m[0], [out[0][0], out[1][0], out[2][0]]))?;
    if determinant.abs() <= 1e-6_f32 {
        return Err(Error::SingularTransform);
    }
    let factor = scalar(1.0_f32 / determinant)?;
    for row in &mut out {
        for value in row {
            *value = scalar(*value * factor)?;
        }
    }
    Ok(out)
}
