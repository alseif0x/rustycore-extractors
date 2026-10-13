// SPDX-License-Identifier: GPL-3.0-or-later
// Authored 2026-10-11: bounded static-manager query adaptation from pinned facts.
//! Manager coordinate and segment operations for the immutable loaded store.
//!
//! Reference 02245dcd245e7433e524577656177723d3e4992e,
//! VMapManager.cpp100-108/172-239 and MapTree.cpp95-186. World positions reflect
//! X/Y once before queries. Internal entry points support callers that already
//! applied that reflection, including RustyCore's LOS interface. Object-hit
//! loaded misses deliberately reflect twice when using the world entry point.
//! The default original per-map disable callback is assumed; global options
//! are honored. This does not install a RustyCore provider or supply liquids.
//!
//! Supplied positions and modify distances must be finite. Public height limits
//! admit nonnegative values and +infinity. LOS overflow retains the original
//! false result; object-hit invalid/overflowing lengths and nonfinite derived
//! positions are explicit errors instead of relying on native assertions.
use super::{Error, Source, StaticStore};
use crate::query::{self, Budget, Mode, Ray};

/// Exact f32 result of the original `(0.5f * 64.0f) * 533.33333333f`.
pub const WORLD_MIDPOINT: f32 = f32::from_bits(0x4685_5555);
pub const INVALID_HEIGHT: f32 = -200_000.0;

/// Complete hit-position result without geometry-bearing Debug output.
#[derive(Clone, Copy)]
pub struct ObjectHit {
    hit: bool,
    position: [f32; 3],
}
impl ObjectHit {
    pub const fn hit(self) -> bool {
        self.hit
    }
    pub const fn position(self) -> [f32; 3] {
        self.position
    }
    pub fn position_bits(self) -> [u32; 3] {
        self.position.map(f32::to_bits)
    }
}
fn finite(v: [f32; 3]) -> Result<[f32; 3], Error> {
    if v.iter().all(|v| v.is_finite()) {
        Ok(v)
    } else {
        Err(Error::Query(query::Error::NonFinite))
    }
}
fn reflect(p: [f32; 3]) -> [f32; 3] {
    [WORLD_MIDPOINT - p[0], WORLD_MIDPOINT - p[1], p[2]]
}
fn delta(end: [f32; 3], start: [f32; 3]) -> [f32; 3] {
    std::array::from_fn(|i| end[i] - start[i])
}
fn length(v: [f32; 3]) -> f32 {
    ((v[0] * v[0] + v[1] * v[1]) + v[2] * v[2]).sqrt()
}
fn advance(start: [f32; 3], direction: [f32; 3], distance: f32) -> [f32; 3] {
    std::array::from_fn(|i| start[i] + direction[i] * distance)
}

impl<S: Source> StaticStore<S> {
    pub fn line_of_sight_world(
        &self,
        map: u32,
        start: [f32; 3],
        end: [f32; 3],
        ignore_m2: bool,
        budget: &mut Budget,
    ) -> Result<bool, Error> {
        self.query_ready(map)?;
        finite(start)?;
        finite(end)?;
        self.line_of_sight_internal(map, reflect(start), reflect(end), ignore_m2, budget)
    }
    pub fn line_of_sight_internal(
        &self,
        map: u32,
        start: [f32; 3],
        end: [f32; 3],
        ignore_m2: bool,
        budget: &mut Budget,
    ) -> Result<bool, Error> {
        self.query_ready(map)?;
        finite(start)?;
        finite(end)?;
        if !self.options().enable_line_of_sight || !self.contains_map(map) || start == end {
            return Ok(true);
        }
        let offset = delta(end, start);
        let distance = length(offset);
        if distance == f32::MAX || !distance.is_finite() {
            return Ok(false);
        }
        if distance < 1e-10_f32 {
            return Ok(true);
        }
        let inverse = 1.0_f32 / distance;
        let direction = offset.map(|v| v * inverse);
        let ray = Ray::new(start, direction).map_err(Error::Query)?;
        Ok(self
            .intersect(map, &ray, distance, Mode::Any, ignore_m2, budget)?
            .is_none())
    }
    pub fn height_world(
        &self,
        map: u32,
        position: [f32; 3],
        maximum: f32,
        budget: &mut Budget,
    ) -> Result<Option<f32>, Error> {
        self.query_ready(map)?;
        finite(position)?;
        self.height_internal(map, reflect(position), maximum, budget)
    }
    pub fn height_internal(
        &self,
        map: u32,
        position: [f32; 3],
        maximum: f32,
        budget: &mut Budget,
    ) -> Result<Option<f32>, Error> {
        self.query_ready(map)?;
        finite(position)?;
        query::validate_distance(maximum).map_err(Error::Query)?;
        if !self.options().enable_height || !self.contains_map(map) {
            return Ok(None);
        }
        let ray = Ray::new(position, [0.0, 0.0, -1.0]).map_err(Error::Query)?;
        let Some(hit) = self.intersect(map, &ray, maximum, Mode::Nearest, false, budget)? else {
            return Ok(None);
        };
        let height = position[2] - hit.hit().distance();
        if !height.is_finite() {
            return Err(Error::Query(query::Error::NonFiniteResult));
        }
        Ok(Some(height))
    }
    pub fn object_hit_world(
        &self,
        map: u32,
        start: [f32; 3],
        end: [f32; 3],
        modify: f32,
        budget: &mut Budget,
    ) -> Result<ObjectHit, Error> {
        self.query_ready(map)?;
        finite(start)?;
        finite(end)?;
        if !modify.is_finite() {
            return Err(Error::Query(query::Error::NonFinite));
        }
        if !self.options().enable_line_of_sight || !self.contains_map(map) {
            return Ok(ObjectHit {
                hit: false,
                position: end,
            });
        }
        let result = self.object_hit_internal(map, reflect(start), reflect(end), modify, budget)?;
        Ok(ObjectHit {
            hit: result.hit,
            position: finite(reflect(result.position))?,
        })
    }
    pub fn object_hit_internal(
        &self,
        map: u32,
        start: [f32; 3],
        end: [f32; 3],
        modify: f32,
        budget: &mut Budget,
    ) -> Result<ObjectHit, Error> {
        self.query_ready(map)?;
        finite(start)?;
        finite(end)?;
        if !modify.is_finite() {
            return Err(Error::Query(query::Error::NonFinite));
        }
        if !self.options().enable_line_of_sight || !self.contains_map(map) {
            return Ok(ObjectHit {
                hit: false,
                position: end,
            });
        }
        let offset = delta(end, start);
        let distance = length(offset);
        if !distance.is_finite() || distance >= f32::MAX {
            return Err(Error::Query(query::Error::InvalidDistance));
        }
        if distance < 1e-10_f32 {
            return Ok(ObjectHit {
                hit: false,
                position: end,
            });
        }
        let inverse = 1.0_f32 / distance;
        let direction = offset.map(|v| v * inverse);
        let ray = Ray::new(start, direction).map_err(Error::Query)?;
        let Some(hit) = self.intersect(map, &ray, distance, Mode::Nearest, false, budget)? else {
            return Ok(ObjectHit {
                hit: false,
                position: end,
            });
        };
        let mut position = advance(start, direction, hit.hit().distance());
        if modify < 0.0 {
            if length(delta(position, start)) > -modify {
                position = advance(position, direction, modify);
            } else {
                position = start;
            }
        } else {
            position = advance(position, direction, modify);
        }
        if position.iter().any(|v| !v.is_finite()) {
            return Err(Error::Query(query::Error::NonFiniteResult));
        }
        Ok(ObjectHit {
            hit: true,
            position,
        })
    }
}

#[cfg(test)]
mod tests;
