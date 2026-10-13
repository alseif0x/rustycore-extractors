// SPDX-License-Identifier: GPL-3.0-or-later
// Authored 2026-10-11: synthetic fresh-tile save tests.
use super::*;
fn params() -> crate::mmap::Params {
    crate::mmap::Params {
        origin: [0.0; 3],
        tile_width: 4.0,
        tile_height: 4.0,
        max_tiles: 1,
        max_polys: i32::MIN,
    }
}
fn seal(m: &TileMesh, bv: bool, direction: u8) -> SavedData {
    let c = connections(&[[[1.0, 0.6, 1.0], [3.0, 0.6, 3.0]]], 0.5, 11, 1, direction);
    let mut o = options(m);
    o.build_bv_tree = bv;
    build(m, c.values(), o, Limits::default())
        .unwrap()
        .prepare_for_save(params(), SaveLimits::default())
        .unwrap()
}
#[test]
fn starts_and_ends_snap_and_link_in_both_bv_modes() {
    for bv in [false, true] {
        for dir in [0, 1] {
            let b = seal(&mesh(), bv, dir);
            let s = b.statistics();
            assert_eq!(s.snapped_endpoints, 2);
            assert_eq!(s.links, 3 + usize::from(dir != 0));
            let nv = word(b.bytes(), 28) as usize;
            let pa = 100 + nv * 12;
            let off = pa + 32;
            for i in 0..2 {
                let vi = half(b.bytes(), off + 4 + i * 2) as usize;
                assert_eq!(f32::from_bits(word(b.bytes(), 100 + vi * 12 + 4)), 0.5);
            }
        }
    }
}
#[test]
fn initializes_every_unused_link_and_poly_head() {
    let d = generate(&mesh());
    let raw = d.bytes().to_vec();
    let b = d.prepare_for_save(params(), SaveLimits::default()).unwrap();
    assert_eq!(b.statistics().links, 0);
    let pa = 100 + word(&raw, 28) as usize * 12;
    let la = pa + word(&raw, 24) as usize * 32;
    let n = word(&raw, 32) as usize;
    assert_eq!(word(b.bytes(), pa), u32::MAX);
    for i in 0..n {
        assert_eq!(
            word(b.bytes(), la + i * 16 + 8),
            if i + 1 == n { u32::MAX } else { i as u32 + 1 }
        );
    }
    let encoded = crate::mmap::encode_saved_tile(&b, true, crate::mmap::Limits::default()).unwrap();
    assert_eq!(
        crate::mmap::TileView::parse(&encoded, crate::mmap::Limits::default())
            .unwrap()
            .payload(),
        b.bytes()
    );
}
#[test]
fn rejects_save_budget_params_and_diagnostics() {
    let mut p = params();
    p.max_tiles = 0;
    assert!(matches!(
        generate(&mesh()).prepare_for_save(p, SaveLimits::default()),
        Err(SaveError::Params)
    ));
    for work in [0, SaveLimits::default().work + 1] {
        assert!(matches!(
            generate(&mesh()).prepare_for_save(params(), SaveLimits { work }),
            Err(SaveError::Limit)
        ));
    }
    let mut d = generate(&mesh());
    d.statistics.detail_base_wraps = 1;
    assert!(matches!(
        d.prepare_for_save(params(), SaveLimits::default()),
        Err(SaveError::Diagnostics)
    ));
    let mut d = generate(&mesh());
    d.statistics.neighbor_marker_collisions = 1;
    assert!(matches!(
        d.prepare_for_save(params(), SaveLimits::default()),
        Err(SaveError::Diagnostics)
    ));
}
#[test]
fn rejects_invalid_detail_index_and_nonfinite_vertex() {
    let mut d = generate(&mesh());
    let b = &d.bytes;
    let tri = 100
        + word(b, 28) as usize * 12
        + word(b, 24) as usize * 32
        + word(b, 32) as usize * 16
        + word(b, 36) as usize * 12
        + word(b, 40) as usize * 12;
    d.bytes[tri] = 255;
    assert!(matches!(
        d.prepare_for_save(params(), SaveLimits::default()),
        Err(SaveError::Topology)
    ));
    let mut d = generate(&mesh());
    d.bytes[100..104].copy_from_slice(&f32::NAN.to_le_bytes());
    assert!(matches!(
        d.prepare_for_save(params(), SaveLimits::default()),
        Err(SaveError::Numeric)
    ));
}
#[test]
fn outside_end_stays_unsnapped_and_keeps_start_backlink() {
    let m = mesh();
    let c = connections(&[[[1.0, 0.6, 1.0], [8.0, 0.6, 3.0]]], 0.5, 11, 1, 1);
    let b = build(&m, c.values(), options(&m), Limits::default())
        .unwrap()
        .prepare_for_save(params(), SaveLimits::default())
        .unwrap();
    assert_eq!(b.statistics().links, 2);
    assert_eq!(b.statistics().snapped_endpoints, 1);
}
#[test]
#[ignore = "requires explicit external synthetic payload and output directories"]
fn write_save_corpus() {
    use std::{env, fs, path::PathBuf};
    let input = PathBuf::from(env::var_os("RCE_SAVE_INPUT").expect("input"));
    let output = PathBuf::from(env::var_os("RCE_SAVE_OUTPUT").expect("output"));
    fs::create_dir(&output).unwrap();
    let mut paths = fs::read_dir(input)
        .unwrap()
        .map(|p| p.unwrap().path())
        .collect::<Vec<_>>();
    paths.sort();
    for p in paths {
        let request = fs::read(&p).unwrap();
        assert!(request.len() >= 108);
        let d = Data {
            bytes: request[8..].to_vec(),
            statistics: Statistics {
                detail_base_wraps: word(&request, 0),
                neighbor_marker_collisions: word(&request, 4),
                ..Statistics::default()
            },
        };
        let out = output.join(p.file_stem().unwrap());
        fs::create_dir(&out).unwrap();
        match d.prepare_for_save(params(), SaveLimits::default()) {
            Ok(saved) => {
                fs::write(out.join("payload.bin"), saved.bytes()).unwrap();
                let b =
                    crate::mmap::encode_saved_tile(&saved, true, crate::mmap::Limits::default())
                        .unwrap();
                fs::write(out.join("tile.bin"), b).unwrap();
                let s = saved.statistics();
                fs::write(
                    out.join("status.txt"),
                    format!("ok {} {} {}\n", s.links, s.snapped_endpoints, s.work),
                )
                .unwrap();
            }
            Err(e) => fs::write(out.join("status.txt"), format!("{}\n", e.code())).unwrap(),
        }
    }
}
#[test]
fn save_work_limit_is_exact_and_only_reduces() {
    let m = mesh();
    let b = seal(&m, true, 1);
    let work = b.statistics().work;
    let c = connections(&[[[1.0, 0.6, 1.0], [3.0, 0.6, 3.0]]], 0.5, 11, 1, 1);
    for (limit, ok) in [(work, true), (work - 1, false)] {
        let out = build(&m, c.values(), options(&m), Limits::default())
            .unwrap()
            .prepare_for_save(params(), SaveLimits { work: limit });
        assert_eq!(out.is_ok(), ok);
        if let Err(e) = out {
            assert_eq!(e, SaveError::Limit);
        }
    }
}
#[test]
fn boundary_projection_and_horizontal_radius_recheck() {
    for (end, rad, snapped) in [([2.5, 0.6, 1.0], 0.5, 2), ([2.5, 0.6, 2.5], 0.5, 1)] {
        let mut m = mesh();
        for v in &mut m.vertices {
            v[0] /= 2;
            v[2] /= 2;
        }
        for bv in [false, true] {
            let c = connections(&[[[1.0, 0.6, 1.0], end]], rad, 11, 1, 1);
            let mut o = options(&m);
            o.build_bv_tree = bv;
            let b = build(&m, c.values(), o, Limits::default())
                .unwrap()
                .prepare_for_save(params(), SaveLimits::default())
                .unwrap();
            assert_eq!(b.statistics().snapped_endpoints, snapped);
        }
    }
}
#[test]
fn degenerate_height_falls_back_to_detail_edges() {
    let mut m = mesh();
    m.detail_triangles = vec![[0, 0, 0, 1]];
    m.submeshes[0].triangle_count = 1;
    let b = seal(&m, true, 1);
    assert_eq!(b.statistics().snapped_endpoints, 2);
}
#[test]
fn missing_boundary_edges_rejects_instead_of_native_null_dereference() {
    let mut m = mesh();
    for v in &mut m.vertices {
        v[0] /= 2;
        v[2] /= 2;
    }
    for t in &mut m.detail_triangles {
        t[3] = 0;
    }
    let c = connections(&[[[1.0, 0.6, 1.0], [2.5, 0.6, 1.0]]], 1.0, 11, 1, 1);
    assert!(matches!(
        build(&m, c.values(), options(&m), Limits::default())
            .unwrap()
            .prepare_for_save(params(), SaveLimits::default()),
        Err(SaveError::Topology)
    ));
}
#[test]
fn nearest_retains_first_tie_and_128_candidate_cap() {
    for high_count in [127, 128] {
        let mut m = mesh();
        m.vertices = vec![
            [0, 4, 0],
            [0, 4, 2],
            [2, 4, 2],
            [2, 4, 0],
            [0, 1, 0],
            [0, 1, 2],
            [2, 1, 2],
            [2, 1, 0],
        ];
        let high = m.polygons[0];
        let mut low = high;
        low.vertices = [4, 5, 6, 7, 65535, 65535];
        m.polygons = vec![high; high_count];
        m.polygons.push(low);
        m.submeshes = vec![m.submeshes[0]; high_count + 1];
        m.header.maximum_polygons = high_count + 1;
        let c = connections(&[[[3.0, 0.6, 1.0], [3.0, 0.6, 1.5]]], 2.0, 11, 1, 1);
        let mut o = options(&m);
        o.walkable_climb = 2.0;
        o.build_bv_tree = false;
        let b = build(&m, c.values(), o, Limits::default())
            .unwrap()
            .prepare_for_save(params(), SaveLimits::default())
            .unwrap();
        assert_eq!(b.statistics().snapped_endpoints, 2);
        let pa = 100 + word(b.bytes(), 28) as usize * 12;
        let off = pa + (high_count + 1) * 32;
        let vi = half(b.bytes(), off + 4) as usize;
        assert_eq!(
            f32::from_bits(word(b.bytes(), 100 + vi * 12 + 4)),
            if high_count == 127 { 0.5 } else { 2.0 }
        );
    }
}
