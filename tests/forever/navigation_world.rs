// SPDX-License-Identifier: GPL-3.0-or-later
// Authored 2026-10-11: strict Forever adapter to unchanged public RustyCore APIs.
// Standalone bounded synthetic QA, built externally with pinned local sources.
// No runtime bootstrap, source acquisition, legacy reader or client-data output.
use std::{collections::BTreeSet, env, fs, path::Path};
use wow_recastdetour::{
    DT_POLY_BITS_LIKE_CPP, DT_TILE_BITS_LIKE_CPP, DetourNavMesh, DetourNavMeshParams,
    DetourNavMeshQuery, DetourNavMeshQueryError, DetourQueryFilter, DetourTileError, MmapTileBlob,
    MmapTileHeader,
};
fn word(b: &[u8], i: usize) -> u32 {
    u32::from_le_bytes(b[i..i + 4].try_into().unwrap())
}
fn reference(out: &mut Vec<u32>, r: u64) {
    out.extend([r as u32, (r >> 32) as u32]);
}
fn tile_index(r: u64) -> u64 {
    (r >> DT_POLY_BITS_LIKE_CPP) & ((1 << DT_TILE_BITS_LIKE_CPP) - 1)
}
#[derive(Clone, Copy, Debug, PartialEq)]
struct Pair {
    a: [f32; 3],
    b: [f32; 3],
}
fn pairs(b: &[u8]) -> Result<Vec<Pair>, &'static str> {
    if b.len() < 12 || word(b, 0) != 0x31514e52 {
        return Err("query header");
    }
    let n = word(b, 4) as usize;
    if !(1..=64).contains(&n) || b.len() != 12 + n * 24 {
        return Err("query count");
    }
    let mut out = Vec::with_capacity(n);
    for i in 0..n {
        let a = std::array::from_fn(|j| f32::from_bits(word(b, 12 + i * 24 + j * 4)));
        let c = std::array::from_fn(|j| f32::from_bits(word(b, 24 + i * 24 + j * 4)));
        if !a.into_iter().chain(c).all(f32::is_finite) {
            return Err("query numeric");
        }
        out.push(Pair { a, b: c });
    }
    Ok(out)
}
fn parameters(b: &[u8], id: u32) -> DetourNavMeshParams {
    let p = forever_navigation::mmap::GlobalView::parse(b, id, Default::default())
        .unwrap()
        .params();
    assert!((1..=4096).contains(&p.max_tiles));
    DetourNavMeshParams {
        origin: p.origin,
        tile_width: p.tile_width,
        tile_height: p.tile_height,
        max_tiles: p.max_tiles,
        max_polys: p.max_polys,
    }
}
fn read(p: &Path, limit: u64) -> Vec<u8> {
    assert!(fs::metadata(p).unwrap().len() <= limit);
    let b = fs::read(p).unwrap();
    assert!(b.len() as u64 <= limit);
    b
}
fn queries(mesh: &DetourNavMesh, pairs: &[Pair]) -> Vec<u32> {
    let query = DetourNavMeshQuery::new(mesh, 4096).unwrap();
    let mut filter = DetourQueryFilter::new().unwrap();
    filter.set_include_flags(65535);
    filter.set_exclude_flags(0);
    // Public APIs expose failures and values; successful native detail flags
    // (e.g. PARTIAL_RESULT) are deliberately not claimed through this adapter.
    let mut out = vec![0, pairs.len() as u32];
    for p in pairs {
        let nearest = [
            query.find_nearest_poly(p.a, [1., 6., 1.], &filter).unwrap(),
            query.find_nearest_poly(p.b, [1., 6., 1.], &filter).unwrap(),
        ];
        for n in &nearest {
            out.push(0);
            reference(&mut out, n.poly_ref);
            out.extend(n.nearest_point.map(f32::to_bits));
        }
        let path = match query.find_path(
            nearest[0].poly_ref,
            nearest[1].poly_ref,
            nearest[0].nearest_point,
            nearest[1].nearest_point,
            &filter,
            4096,
        ) {
            Ok(path) => {
                out.push(0);
                path
            }
            Err(DetourNavMeshQueryError::FindPathFailed { status }) => {
                out.push(status);
                Vec::new()
            }
            Err(e) => panic!("unexpected path error: {e}"),
        };
        out.push(path.len() as u32);
        for &r in &path {
            reference(&mut out, r);
        }
        let straight = match query.find_straight_path(
            nearest[0].nearest_point,
            nearest[1].nearest_point,
            &path,
            4096,
            0,
        ) {
            Ok(path) => {
                out.push(0);
                path
            }
            Err(DetourNavMeshQueryError::FindStraightPathFailed { status }) => {
                out.push(status);
                Vec::new()
            }
            Err(e) => panic!("unexpected straight-path error: {e}"),
        };
        out.push(straight.len() as u32);
        for s in straight {
            out.extend(s.position.map(f32::to_bits));
            out.push(u32::from(s.flags));
            reference(&mut out, s.poly_ref);
        }
        out.push(u32::from(
            !path.is_empty() && path.last() == Some(&nearest[1].poly_ref),
        ));
        out.push(
            path.iter()
                .map(|&r| tile_index(r))
                .collect::<BTreeSet<_>>()
                .len() as u32,
        );
    }
    out
}
fn add(mesh: &mut DetourNavMesh, tile: &MmapTileBlob, out: &mut Vec<u32>) -> u64 {
    match mesh.add_tile(tile) {
        Ok(r) => {
            out.push(0);
            reference(out, r);
            r
        }
        Err(DetourTileError::AddTileFailed { status }) => {
            out.push(status);
            reference(out, 0);
            0
        }
        Err(e) => panic!("unexpected add error: {e}"),
    }
}
fn remove(mesh: &mut DetourNavMesh, r: u64, out: &mut Vec<u32>) {
    match mesh.remove_tile(r) {
        Ok(()) => out.push(0),
        Err(DetourTileError::RemoveTileFailed { status }) => out.push(status),
        Err(e) => panic!("unexpected remove error: {e}"),
    }
}
fn main() {
    let args: Vec<_> = env::args().collect();
    assert_eq!(args.len(), 5);
    let plan = Path::new(&args[1]);
    let root = plan.parent().unwrap();
    let text = String::from_utf8(read(plan, 65536)).unwrap();
    let mut words = text.split_whitespace();
    let global = root.join(words.next().unwrap());
    let id: u32 = global
        .file_stem()
        .unwrap()
        .to_str()
        .unwrap()
        .parse()
        .unwrap();
    let count: usize = words.next().unwrap().parse().unwrap();
    assert!(count <= 16);
    let params = parameters(&read(&global, 64 * 1024 * 1024), id);
    let mut tiles = Vec::with_capacity(count);
    let mut total = 0;
    for _ in 0..count {
        let path = root.join(words.next().unwrap());
        let b = read(&path, 64 * 1024 * 1024);
        total += b.len();
        assert!(total <= 128 * 1024 * 1024);
        let view = forever_navigation::mmap::TileView::parse(&b, Default::default()).unwrap();
        assert!(MmapTileBlob::parse(&b, 7).is_err());
        tiles.push(MmapTileBlob {
            header: MmapTileHeader {
                mmap_magic: 0x4d4d4150,
                dt_version: 7,
                mmap_version: 16,
                size: view.payload().len() as u32,
                uses_liquids: view.uses_liquids(),
                padding: [0; 3],
            },
            data: view.payload().to_vec(),
        });
    }
    assert!(words.next().is_none());
    let positions = read(Path::new(&args[3]), 65536);
    let positions = pairs(&positions).unwrap();
    let mut mesh = DetourNavMesh::new(&params).unwrap();
    let mut out = vec![0x31574e52, 0, mesh.max_tiles(), count as u32];
    assert_eq!(mesh.max_tiles(), params.max_tiles as u32);
    let mut refs: Vec<_> = tiles.iter().map(|t| add(&mut mesh, t, &mut out)).collect();
    out.extend(queries(&mesh, &positions));
    if let Some(i) = refs.iter().position(|&r| r != 0) {
        out.push(1);
        let old = refs[i];
        assert_eq!(add(&mut mesh, &tiles[i], &mut out), 0);
        remove(&mut mesh, old, &mut out);
        refs[i] = 0;
        out.extend(queries(&mesh, &positions));
        refs[i] = add(&mut mesh, &tiles[i], &mut out);
        assert_ne!(refs[i], 0);
        assert_ne!(refs[i], old);
        out.extend(queries(&mesh, &positions));
        remove(&mut mesh, old, &mut out);
    } else {
        out.push(0);
    }
    out.push(refs.iter().filter(|&&r| r != 0).count() as u32);
    for r in refs.into_iter().rev().filter(|&r| r != 0) {
        remove(&mut mesh, r, &mut out);
    }
    out.extend(queries(&mesh, &positions));
    let bytes: Vec<_> = out.iter().flat_map(|n| n.to_le_bytes()).collect();
    let oracle = read(Path::new(&args[2]), 64 * 1024 * 1024);
    assert_eq!(bytes, oracle, "multi-tile public API result differs");
    assert!(!Path::new(&args[4]).exists());
    fs::write(&args[4], &bytes).unwrap();
    println!(
        "matched {} queries over full/remove/reload/empty lifecycle, {} input tiles",
        positions.len(),
        count
    );
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn fixed_reference_tile_bits_ignore_salt_and_polygon() {
        let r = (7u64 << 52) | (123u64 << 31) | 456;
        assert_eq!(tile_index(r), 123);
        assert_eq!(tile_index(0), 0);
    }
    #[test]
    fn query_plan_checks_framing_and_finite_positions() {
        let mut b = vec![0; 36];
        b[..4].copy_from_slice(&0x31514e52u32.to_le_bytes());
        b[4..8].copy_from_slice(&1u32.to_le_bytes());
        assert_eq!(
            pairs(&b).unwrap(),
            vec![Pair {
                a: [0.; 3],
                b: [0.; 3]
            }]
        );
        assert!(pairs(&b[..35]).is_err());
        b[12..16].copy_from_slice(&f32::NAN.to_bits().to_le_bytes());
        assert!(pairs(&b).is_err());
        assert!(pairs(&[]).is_err());
        b[4..8].copy_from_slice(&65u32.to_le_bytes());
        assert!(pairs(&b).is_err());
    }
    #[test]
    fn global_adapter_keeps_original_capacity_and_signed_poly_sentinel() {
        let p = forever_navigation::mmap::Params {
            origin: [1., f32::MIN_POSITIVE, 2.],
            tile_width: 533.3333,
            tile_height: 533.3333,
            max_tiles: 4,
            max_polys: i32::MIN,
        };
        let b = forever_navigation::mmap::encode_global(1, p, &[], Default::default()).unwrap();
        let actual = parameters(&b, 1);
        assert_eq!(actual.max_tiles, 4);
        assert_eq!(actual.max_polys, i32::MIN);
        assert_eq!(actual.origin.map(f32::to_bits), p.origin.map(f32::to_bits));
    }
}
