// SPDX-License-Identifier: GPL-3.0-or-later
//
// Copyright (c) 2009-2010 Mikko Mononen memon@inside.org
//
// This software is provided 'as-is', without any express or implied
// warranty.  In no event will the authors be held liable for any damages
// arising from the use of this software.
// Permission is granted to anyone to use this software for any purpose,
// including commercial applications, and to alter it and redistribute it
// freely, subject to the following restrictions:
// 1. The origin of this software must not be misrepresented; you must not
//    claim that you wrote the original software. If you use this software
//    in a product, an acknowledgment in the product documentation would be
//    appreciated but is not required.
// 2. Altered source versions must be plainly marked as such, and must not be
//    misrepresented as being the original software.
// 3. This notice may not be removed or altered from any source distribution.
//

// Modified/ported 2026-10-11: Detour quantization and bounded stable BV ordering.
use super::*;
#[derive(Clone, Copy, Default)]
struct Item {
    min: [u16; 3],
    max: [u16; 3],
    index: i32,
}
fn quantize(value: f32, origin: f32, q: f32) -> Result<u16, Error> {
    let n = (value - origin) * q;
    if !n.is_finite() || f64::from(n) < f64::from(i32::MIN) || f64::from(n) > f64::from(i32::MAX) {
        return Err(Error::Numeric);
    }
    Ok((n as i32).clamp(0, 65535) as u16)
}
fn stable_sort(
    b: &mut Budget,
    items: &mut [Item],
    scratch: &mut [Item],
    axis: usize,
) -> Result<(), Error> {
    if items.len() < 2 {
        return Ok(());
    }
    let mid = items.len() / 2;
    stable_sort(b, &mut items[..mid], &mut scratch[..mid], axis)?;
    stable_sort(b, &mut items[mid..], &mut scratch[mid..], axis)?;
    let (mut left, mut right) = (0, mid);
    for slot in scratch.iter_mut().take(items.len()) {
        b.charge(1)?;
        if left < mid && (right == items.len() || items[left].min[axis] <= items[right].min[axis]) {
            *slot = items[left];
            left += 1;
        } else {
            *slot = items[right];
            right += 1;
        }
    }
    b.charge(items.len() as u64)?;
    items.copy_from_slice(&scratch[..items.len()]);
    Ok(())
}
fn subdivide(
    b: &mut Budget,
    items: &mut [Item],
    scratch: &mut [Item],
    out: &mut [u8],
    start: usize,
    next: &mut usize,
) -> Result<(), Error> {
    b.charge(1)?;
    let current = *next;
    *next += 1;
    let mut node = items[0];
    if items.len() > 1 {
        for item in &items[1..] {
            b.charge(1)?;
            for a in 0..3 {
                node.min[a] = node.min[a].min(item.min[a]);
                node.max[a] = node.max[a].max(item.max[a]);
            }
        }
        let lengths = [
            node.max[0] - node.min[0],
            node.max[1] - node.min[1],
            node.max[2] - node.min[2],
        ];
        let mut axis = 0;
        if lengths[1] > lengths[0] {
            axis = 1;
        }
        if lengths[2] > lengths[axis] {
            axis = 2;
        }
        stable_sort(b, items, scratch, axis)?;
        let mid = items.len() / 2;
        subdivide(b, &mut items[..mid], &mut scratch[..mid], out, start, next)?;
        subdivide(b, &mut items[mid..], &mut scratch[mid..], out, start, next)?;
        node.index = -((*next - current) as i32);
    }
    let at = start + current * 16;
    for a in 0..3 {
        encode::half(out, at + a * 2, node.min[a]);
        encode::half(out, at + 6 + a * 2, node.max[a]);
    }
    encode::word(out, at + 12, node.index as u32);
    Ok(())
}
pub(super) fn write(
    b: &mut Budget,
    out: &mut [u8],
    start: usize,
    m: &TileMesh,
    origin: Point,
    q: f32,
) -> Result<(), Error> {
    let mut items = b.filled(m.polygons().len(), Item::default())?;
    for (i, s) in m.submeshes().iter().enumerate() {
        b.charge(1)?;
        let vertices =
            &m.detail_vertices()[s.vertex_base as usize..(s.vertex_base + s.vertex_count) as usize];
        let mut min = vertices[0];
        let mut max = min;
        for v in &vertices[1..] {
            b.charge(1)?;
            for a in 0..3 {
                if min[a] >= v[a] {
                    min[a] = v[a];
                }
                if max[a] <= v[a] {
                    max[a] = v[a];
                }
            }
        }
        for a in 0..3 {
            items[i].min[a] = quantize(min[a], origin[a], q)?;
            items[i].max[a] = quantize(max[a], origin[a], q)?;
        }
        items[i].index = i as i32;
    }
    let mut scratch = b.filled(items.len(), Item::default())?;
    let mut next = 0;
    subdivide(b, &mut items, &mut scratch, out, start, &mut next)?;
    assert_eq!(next, m.polygons().len() * 2 - 1);
    Ok(())
}
