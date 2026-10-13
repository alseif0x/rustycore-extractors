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

// Modified/ported 2026-10-11: Detour payload fields, explicit 64-bit-ref ABI.
use super::*;
pub(super) struct Layout {
    pub bytes: usize,
    vertices: usize,
    polygons: usize,
    details: usize,
    extra: usize,
    triangles: usize,
    pub tree: usize,
    offmesh: usize,
    total_vertices: usize,
    total_polygons: usize,
}
impl Layout {
    pub fn new(
        m: &TileMesh,
        nv: usize,
        np: usize,
        s: &Statistics,
        tree: bool,
    ) -> Result<Self, Error> {
        let mut at = 100usize;
        let mut block = |count: usize, size: usize| -> Result<usize, Error> {
            let start = at;
            at = at
                .checked_add(count.checked_mul(size).ok_or(Error::Limit)?)
                .ok_or(Error::Limit)?;
            Ok(start)
        };
        let vertices = block(nv, 12)?;
        let polygons = block(np, 32)?;
        block(s.maximum_links, 16)?;
        let details = block(m.polygons().len(), 12)?;
        let extra = block(s.extra_detail_vertices, 12)?;
        let triangles = block(m.detail_triangles().len(), 4)?;
        let tree = block(if tree { m.polygons().len() * 2 } else { 0 }, 16)?;
        let offmesh = block(s.stored_connections, 36)?;
        Ok(Self {
            bytes: at,
            vertices,
            polygons,
            details,
            extra,
            triangles,
            tree,
            offmesh,
            total_vertices: nv,
            total_polygons: np,
        })
    }
}
pub(super) fn word(out: &mut [u8], at: usize, n: u32) {
    out[at..at + 4].copy_from_slice(&n.to_le_bytes());
}
pub(super) fn half(out: &mut [u8], at: usize, n: u16) {
    out[at..at + 2].copy_from_slice(&n.to_le_bytes());
}
fn point(out: &mut [u8], at: usize, p: Point) {
    for (i, v) in p.into_iter().enumerate() {
        word(out, at + i * 4, v.to_bits());
    }
}
pub(super) fn write(
    out: &mut [u8],
    l: &Layout,
    m: &TileMesh,
    connections: &[Connection],
    classes: &[[u8; 2]],
    o: Options,
    s: &mut Statistics,
) -> Result<(), Error> {
    let q = finite(1.0 / m.header().cell_size)?;
    let header = [
        MAGIC,
        VERSION,
        o.tile[0] as u32,
        o.tile[1] as u32,
        o.tile[2] as u32,
        o.user_id,
        l.total_polygons as u32,
        l.total_vertices as u32,
        s.maximum_links as u32,
        m.polygons().len() as u32,
        s.extra_detail_vertices as u32,
        m.detail_triangles().len() as u32,
        if o.build_bv_tree {
            m.polygons().len() as u32 * 2
        } else {
            0
        },
        s.stored_connections as u32,
        m.polygons().len() as u32,
        o.walkable_height.to_bits(),
        o.walkable_radius.to_bits(),
        o.walkable_climb.to_bits(),
        o.min[0].to_bits(),
        o.min[1].to_bits(),
        o.min[2].to_bits(),
        o.max[0].to_bits(),
        o.max[1].to_bits(),
        o.max[2].to_bits(),
        q.to_bits(),
    ];
    for (i, v) in header.into_iter().enumerate() {
        word(out, i * 4, v);
    }
    let h = m.header();
    for (i, v) in m.vertices().iter().enumerate() {
        point(
            out,
            l.vertices + i * 12,
            [
                finite(o.min[0] + f32::from(v[0]) * h.cell_size)?,
                finite(o.min[1] + f32::from(v[1]) * h.cell_height)?,
                finite(o.min[2] + f32::from(v[2]) * h.cell_size)?,
            ],
        );
    }
    for (i, p) in m.polygons().iter().enumerate() {
        let at = l.polygons + i * 32;
        for (j, (&v, &n)) in p.vertices().iter().zip(p.neighbors()).enumerate() {
            half(out, at + 4 + j * 2, v);
            let n = if n & 0x8000 != 0 {
                match n & 15 {
                    0 => 0x8004,
                    1 => 0x8002,
                    2 => 0x8000,
                    3 => 0x8006,
                    _ => 0,
                }
            } else {
                n + 1
            };
            half(out, at + 16 + j * 2, n);
        }
        let flags = match o.flags {
            FlagPolicy::Preserve => p.flags(),
            FlagPolicy::Forever => forever_flags(p.area(), p.flags())?,
        };
        half(out, at + 28, flags);
        out[at + 30] = p.vertices().len() as u8;
        out[at + 31] = p.area() & 63;
    }
    let mut vbase = 0u16;
    for (i, (p, d)) in m.polygons().iter().zip(m.submeshes()).enumerate() {
        let extra = d.vertex_count as usize - p.vertices().len();
        let at = l.details + i * 12;
        word(out, at, u32::from(vbase));
        word(out, at + 4, d.triangle_base);
        out[at + 8] = extra as u8;
        out[at + 9] = d.triangle_count as u8;
        for k in 0..extra {
            point(
                out,
                l.extra + (usize::from(vbase) + k) * 12,
                m.detail_vertices()[d.vertex_base as usize + p.vertices().len() + k],
            );
        }
        if usize::from(vbase) + extra > 65535 {
            s.detail_base_wraps += 1;
        }
        vbase = vbase.wrapping_add(extra as u16);
    }
    for (i, t) in m.detail_triangles().iter().enumerate() {
        out[l.triangles + i * 4..l.triangles + (i + 1) * 4].copy_from_slice(t);
    }
    let mut stored = 0usize;
    for (c, class) in connections.iter().zip(classes) {
        if class[0] != 255 {
            continue;
        }
        let vb = m.vertices().len() + stored * 2;
        let pb = m.polygons().len() + stored;
        let ends = c.endpoints();
        point(out, l.vertices + vb * 12, ends[0]);
        point(out, l.vertices + (vb + 1) * 12, ends[1]);
        let p = l.polygons + pb * 32;
        half(out, p + 4, vb as u16);
        half(out, p + 6, (vb + 1) as u16);
        half(out, p + 28, c.flags());
        out[p + 30] = 2;
        out[p + 31] = (c.area() & 63) | 64;
        let at = l.offmesh + stored * 36;
        point(out, at, ends[0]);
        point(out, at + 12, ends[1]);
        word(out, at + 24, c.radius().to_bits());
        half(out, at + 28, pb as u16);
        out[at + 30] = u8::from(c.direction() != 0);
        out[at + 31] = class[1];
        stored += 1;
    }
    Ok(())
}
