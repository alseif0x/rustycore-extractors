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

// Modified/ported 2026-10-11: native longest-edge convex merges and swap order.
use super::*;
fn left(a: [u16; 3], b: [u16; 3], c: [u16; 3]) -> bool {
    (i32::from(b[0]) - i32::from(a[0])) * (i32::from(c[2]) - i32::from(a[2]))
        - (i32::from(c[0]) - i32::from(a[0])) * (i32::from(b[2]) - i32::from(a[2]))
        < 0
}
impl Budget {
    fn merge_value(
        &mut self,
        vertices: &[[u16; 3]],
        a: Polygon,
        b: Polygon,
        nvp: usize,
    ) -> Result<Option<(i32, usize, usize)>, Error> {
        self.charge(1)?;
        let (na, nb) = (a.len(), b.len());
        if na + nb - 2 > nvp {
            return Ok(None);
        }
        let mut shared = None;
        for i in 0..na {
            let (a0, a1) = (a.vertices[i], a.vertices[(i + 1) % na]);
            for j in 0..nb {
                self.charge(1)?;
                let (b0, b1) = (b.vertices[j], b.vertices[(j + 1) % nb]);
                if (a0.min(a1), a0.max(a1)) == (b0.min(b1), b0.max(b1)) {
                    shared = Some((i, j));
                    break;
                }
            }
        }
        let Some((ea, eb)) = shared else {
            return Ok(None);
        };
        let v = |id: u16| vertices[usize::from(id)];
        if !left(
            v(a.vertices[(ea + na - 1) % na]),
            v(a.vertices[ea]),
            v(b.vertices[(eb + 2) % nb]),
        ) || !left(
            v(b.vertices[(eb + nb - 1) % nb]),
            v(b.vertices[eb]),
            v(a.vertices[(ea + 2) % na]),
        ) {
            return Ok(None);
        }
        let p = v(a.vertices[ea]);
        let q = v(a.vertices[(ea + 1) % na]);
        let dx = i32::from(p[0]) - i32::from(q[0]);
        let dz = i32::from(p[2]) - i32::from(q[2]);
        Ok(Some((dx * dx + dz * dz, ea, eb)))
    }
    pub(super) fn merge_polygons(
        &mut self,
        vertices: &[[u16; 3]],
        polys: &mut Vec<Polygon>,
        nvp: usize,
    ) -> Result<(), Error> {
        if nvp <= 3 {
            return Ok(());
        }
        loop {
            let mut best = None;
            for j in 0..polys.len().saturating_sub(1) {
                for k in j + 1..polys.len() {
                    if let Some((value, ea, eb)) =
                        self.merge_value(vertices, polys[j], polys[k], nvp)?
                        && value > best.map(|(v, _, _, _, _)| v).unwrap_or(0)
                    {
                        best = Some((value, j, k, ea, eb));
                    }
                }
            }
            let Some((_, j, k, ea, eb)) = best else { break };
            let (a, b) = (polys[j], polys[k]);
            let (na, nb) = (a.len(), b.len());
            let mut joined = Polygon::new(if a.region == b.region { a.region } else { 0 }, a.area);
            for i in 0..na - 1 {
                self.charge(1)?;
                joined.vertices[i] = a.vertices[(ea + 1 + i) % na];
            }
            for i in 0..nb - 1 {
                self.charge(1)?;
                joined.vertices[na - 1 + i] = b.vertices[(eb + 1 + i) % nb];
            }
            polys[j] = joined;
            self.charge(1)?;
            polys.swap_remove(k);
        }
        Ok(())
    }
}
