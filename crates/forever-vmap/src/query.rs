// SPDX-License-Identifier: GPL-3.0-or-later
// Authored 2026-10-11: original bounded query implementation from operation facts.
// No TrinityCore or G3D implementation is imported.
//! Pure ray queries over completely validated BIH and VMO views.
//!
//! Reference: 02245dcd245e7433e524577656177723d3e4992e,
//! BoundingIntervalHierarchy.h89-229, G3D Ray.cpp18-24/208-218,
//! CollisionDetection.cpp1210-1301 and g3dmath.h825-864. The original BIH
//! credits Sunflow, Christopher Kulla (2003-2007), MIT/X11; TrinityCore's
//! implementation is GPL-2.0-or-later. These files implement the reviewed
//! operations independently under this project's license.
//!
//! Rays retain their supplied direction and signed zeros; no normalization is
//! performed. Distances are ray parameters, physical distances only for unit
//! directions. Finite origin/direction and a nonzero direction are required.
//! Component reciprocals may be infinite, as in native release Ray::set.
//! Public maximum distance admits nonnegative values and positive infinity.
//! Native traversal into an absent infinity-marked child can be unsafe for
//! some infinite-distance inputs. This implementation returns UnsafeTraversal
//! instead of reading a phantom node, inventing a miss or scanning all objects.
//!
//! One Budget is shared by nested model/group/mesh traversals. Work is charged
//! before each node or triangle test; exhaustion is an error, never a miss.
//! The traversal uses at most 65 stack entries per nested tree and no heap.
//! These counters do not measure every arithmetic operation, input validation
//! pass, stack byte or allocator overhead. No file loading or runtime manager
//! is provided here. Floating operations preserve the pinned Linux f32/f64
//! profile without explicit fused operations or approximate math.
pub(crate) mod math;
pub mod model;
#[cfg(test)]
mod tests;
pub(crate) mod traverse;

pub const MAX_NODE_VISITS: u64 = 16_000_000;
pub const MAX_TRIANGLE_TESTS: u64 = 16_000_000;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mode {
    Nearest,
    Any,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Limits {
    pub max_node_visits: u64,
    pub max_triangle_tests: u64,
}
impl Default for Limits {
    fn default() -> Self {
        Self {
            max_node_visits: MAX_NODE_VISITS,
            max_triangle_tests: MAX_TRIANGLE_TESTS,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Stats {
    pub node_visits: u64,
    pub triangle_tests: u64,
}

/// Cumulative work ledger, including successive public calls using this value.
/// Counters are incremented only after their next unit has been admitted.
pub struct Budget {
    limits: Limits,
    stats: Stats,
}
impl Budget {
    pub fn new(limits: Limits) -> Result<Self, Error> {
        if limits.max_node_visits > MAX_NODE_VISITS
            || limits.max_triangle_tests > MAX_TRIANGLE_TESTS
        {
            return Err(Error::InvalidLimits);
        }
        Ok(Self {
            limits,
            stats: Stats::default(),
        })
    }
    pub const fn stats(&self) -> Stats {
        self.stats
    }
    pub(crate) fn charge_node(&mut self) -> Result<(), Error> {
        if self.stats.node_visits >= self.limits.max_node_visits {
            return Err(Error::NodeLimit);
        }
        self.stats.node_visits += 1;
        Ok(())
    }
    pub(crate) fn charge_triangle(&mut self) -> Result<(), Error> {
        if self.stats.triangle_tests >= self.limits.max_triangle_tests {
            return Err(Error::TriangleLimit);
        }
        self.stats.triangle_tests += 1;
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Error {
    InvalidLimits,
    NonFinite,
    ZeroDirection,
    InvalidDistance,
    NodeLimit,
    TriangleLimit,
    InvalidTree,
    StackLimit,
    UnsafeTraversal,
    NonFiniteResult,
    MissingBounds,
    InvalidScale,
    SingularTransform,
    InvalidModel,
}
impl Error {
    pub const fn code(self) -> &'static str {
        match self {
            Self::InvalidLimits => "vmap_query_invalid_limits",
            Self::NonFinite => "vmap_query_nonfinite_ray",
            Self::ZeroDirection => "vmap_query_zero_direction",
            Self::InvalidDistance => "vmap_query_invalid_distance",
            Self::NodeLimit => "vmap_query_node_limit",
            Self::TriangleLimit => "vmap_query_triangle_limit",
            Self::InvalidTree => "vmap_query_invalid_tree",
            Self::StackLimit => "vmap_query_stack_limit",
            Self::UnsafeTraversal => "vmap_query_unsafe_traversal",
            Self::NonFiniteResult => "vmap_query_nonfinite_result",
            Self::MissingBounds => "vmap_query_missing_bounds",
            Self::InvalidScale => "vmap_query_invalid_scale",
            Self::SingularTransform => "vmap_query_singular_transform",
            Self::InvalidModel => "vmap_query_invalid_model",
        }
    }
}
impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.code())
    }
}
impl std::error::Error for Error {}

/// Validated finite ray; deliberately no geometry-bearing Debug output.
#[derive(Clone, Copy)]
pub struct Ray {
    origin: [f32; 3],
    direction: [f32; 3],
    inverse: [f32; 3],
}
impl Ray {
    pub fn new(origin: [f32; 3], direction: [f32; 3]) -> Result<Self, Error> {
        if origin.iter().chain(&direction).any(|v| !v.is_finite()) {
            return Err(Error::NonFinite);
        }
        if direction.iter().all(|v| *v == 0.0) {
            return Err(Error::ZeroDirection);
        }
        Ok(Self {
            origin,
            direction,
            inverse: direction.map(|v| 1.0 / v),
        })
    }
    pub fn from_bits(origin: [u32; 3], direction: [u32; 3]) -> Result<Self, Error> {
        Self::new(origin.map(f32::from_bits), direction.map(f32::from_bits))
    }
    pub const fn origin(&self) -> [f32; 3] {
        self.origin
    }
    pub const fn direction(&self) -> [f32; 3] {
        self.direction
    }
    pub const fn inverse_direction(&self) -> [f32; 3] {
        self.inverse
    }
}

pub(crate) fn validate_distance(distance: f32) -> Result<(), Error> {
    if distance.is_nan() || distance < 0.0 {
        Err(Error::InvalidDistance)
    } else {
        Ok(())
    }
}
