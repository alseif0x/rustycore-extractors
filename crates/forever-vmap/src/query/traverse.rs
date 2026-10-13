// SPDX-License-Identifier: GPL-3.0-or-later
// Authored 2026-10-11: original bounded BIH traversal from pinned operation facts.
use super::{Budget, Error, Mode, Ray, math};
use crate::bih;

#[derive(Clone, Copy)]
struct Pending {
    node: usize,
    near: f32,
    far: f32,
    live: bool,
}
const EMPTY: Pending = Pending {
    node: 0,
    near: 0.0,
    far: 0.0,
    live: true,
};

/// Private because callbacks must preserve the native monotonic distance and
/// result contracts. Public model entry points validate initial distance;
/// negative distance from a negative instance scale is intentionally admitted
/// here, matching native traversal. Never normalize the supplied ray.
pub(crate) fn ray(
    tree: &bih::View<'_>,
    ray: &Ray,
    distance: &mut f32,
    mode: Mode,
    budget: &mut Budget,
    mut callback: impl FnMut(u32, &mut f32, &mut Budget) -> Result<bool, Error>,
) -> Result<bool, Error> {
    let bounds = tree.bounds_bits().map(f32::from_bits);
    let origin = ray.origin();
    let direction = ray.direction();
    let inverse = ray.inverse_direction();
    let mut near = -1.0f32;
    let mut far = -1.0f32;
    for axis in 0..3 {
        if math::fuzzy_nonzero(direction[axis]) {
            let mut a = (bounds[axis] - origin[axis]) * inverse[axis];
            let mut b = (bounds[axis + 3] - origin[axis]) * inverse[axis];
            if a > b {
                std::mem::swap(&mut a, &mut b);
            }
            if a > near {
                near = a;
            }
            if b < far || far < 0.0 {
                far = b;
            }
            if far <= 0.0 || near >= *distance {
                return Ok(false);
            }
        }
    }
    if near > far {
        return Ok(false);
    }
    near = math::first_max(near, 0.0);
    far = math::first_min(far, *distance);
    let signs = direction.map(|v| (v.to_bits() >> 31) as usize);
    let mut stack = [EMPTY; bih::MAX_DEPTH];
    let mut used = 0usize;
    let mut node = 0usize;
    let mut live = true;
    let mut hit = false;
    loop {
        loop {
            // Some native +infinity queries can reach a phantom unary edge.
            // It can alias a real node as well as point outside the wire, so
            // bounds checking alone cannot safely detect this condition.
            if !live {
                return Err(Error::UnsafeTraversal);
            }
            budget.charge_node()?;
            if !node.is_multiple_of(3) {
                return Err(Error::InvalidTree);
            }
            let head = tree.query_word(node).ok_or(Error::InvalidTree)?;
            let axis = (head >> 30) as usize;
            let offset = (head & 0x1fff_ffff) as usize;
            let narrowed = head & 0x2000_0000 != 0;
            let left = tree.query_word(node + 1).ok_or(Error::InvalidTree)?;
            let right = tree.query_word(node + 2).ok_or(Error::InvalidTree)?;
            if axis == 3 && !narrowed {
                for ordinal in offset
                    ..offset
                        .checked_add(left as usize)
                        .ok_or(Error::InvalidTree)?
                {
                    let object = tree.query_object(ordinal).ok_or(Error::InvalidTree)?;
                    let found = callback(object, distance, budget)?;
                    hit |= found;
                    if mode == Mode::Any && found {
                        return Ok(true);
                    }
                }
                break;
            }
            if axis >= 3 {
                return Err(Error::InvalidTree);
            }
            let clips = [f32::from_bits(left), f32::from_bits(right)];
            let front = signs[axis];
            let back = front ^ 1;
            let tf = (clips[front] - origin[axis]) * inverse[axis];
            let tb = (clips[back] - origin[axis]) * inverse[axis];
            if narrowed {
                node = offset;
                near = if tf >= near { tf } else { near };
                far = if tb <= far { tb } else { far };
                if near > far {
                    break;
                }
                continue;
            }
            if tf < near && tb > far {
                break;
            }
            let real = [clips[0] != f32::NEG_INFINITY, clips[1] != f32::INFINITY];
            node = offset + back * 3;
            live = real[back];
            if tf < near {
                near = if tb >= near { tb } else { near };
                continue;
            }
            node = offset + front * 3;
            live = real[front];
            if tb > far {
                far = if tf <= far { tf } else { far };
                continue;
            }
            let pending = stack.get_mut(used).ok_or(Error::StackLimit)?;
            *pending = Pending {
                node: offset + back * 3,
                near: if tb >= near { tb } else { near },
                far,
                live: real[back],
            };
            used += 1;
            far = if tf <= far { tf } else { far };
        }
        loop {
            if used == 0 {
                return Ok(hit);
            }
            used -= 1;
            let pending = stack[used];
            near = pending.near;
            if *distance < near {
                continue;
            }
            node = pending.node;
            far = pending.far;
            live = pending.live;
            break;
        }
    }
}
