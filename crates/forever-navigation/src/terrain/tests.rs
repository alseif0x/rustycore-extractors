// SPDX-License-Identifier: GPL-3.0-or-later
// Authored 2026-10-11: wholly synthetic checked MAPS navigation fixtures.
use super::*;
use forever_terrain::{
    height::{self, HeightSection},
    liquid_section::{self, LiquidSection},
    maps::{self, AreaSection, MapSections},
};

/// Authored arrays only; no client or reference-extracted bytes.
pub(crate) fn fixture(build: u32, kind: u8, liquid_kind: u8, holes: bool) -> Vec<u8> {
    let scale = match kind {
        0 => 0.0,
        1 => 0.0005,
        2 => 0.01,
        _ => 0.25,
    };
    let v9: Vec<_> = (0..V9_COUNT)
        .map(|i| (2.0 + (i % 213) as f32 * scale).to_bits())
        .collect();
    let v8: Vec<_> = (0..V8_COUNT)
        .map(|i| (2.0 + (i % 119) as f32 * scale).to_bits())
        .collect();
    let height = HeightSection::from_bits(
        &v9,
        &v8,
        height::Options {
            pack_integers: kind < 3,
        },
        None,
        height::MAX_SECTION_BYTES,
    )
    .unwrap();
    let area = AreaSection::from_ids(&[42; 256]);
    let mut liquid_flags = vec![
        match liquid_kind {
            0 => 0,
            1..=4 => 1,
            5 => 16,
            6 => 4,
            7 => 8,
            8 => 2,
            9 => 32,
            _ => 1,
        };
        256
    ];
    if liquid_kind == 10 {
        for (i, f) in liquid_flags.iter_mut().enumerate() {
            *f = [0, 1, 2, 4, 8, 16, 32, 17, 5, 255][i % 10];
        }
    }
    let level: f32 = match liquid_kind {
        1 => 100.0,
        2 => -10.0,
        3 => 2.0,
        _ => 3.0,
    };
    let mut show = vec![1; V8_COUNT];
    if liquid_kind == 4 {
        show.fill(0);
        for y in 9..12 {
            for x in 6..11 {
                show[y * 128 + x] = 1;
            }
        }
    }
    if liquid_kind == 11 {
        show.fill(0);
    }
    let mut levels = vec![level.to_bits(); V9_COUNT];
    if liquid_kind == 4 || liquid_kind == 10 {
        for (i, b) in levels.iter_mut().enumerate() {
            *b = [3.0f32, -2000.0, 5000.0, 5001.0, -2001.0, 2.0][i % 6].to_bits();
        }
    }
    let liq = LiquidSection::from_arrays(
        &show,
        &[7; 256],
        &liquid_flags,
        &levels,
        liquid_section::Options {
            pack_integers: false,
        },
        liquid_section::MAX_SECTION_BYTES,
    )
    .unwrap();
    let mut hole_bytes = [0u8; 2048];
    if holes {
        for (i, b) in hole_bytes.iter_mut().enumerate() {
            *b = (i.wrapping_mul(37) & 255) as u8;
        }
    }
    maps::assemble(
        build,
        MapSections {
            area: &area,
            height: &height,
            liquid: liq.as_ref(),
            holes: if holes { Some(&hole_bytes) } else { None },
        },
        maps::MAX_MAP_BYTES,
    )
    .unwrap()
}
fn view(data: &[u8], build: u32) -> MapView<'_> {
    maps::verify(data, build, maps::MAX_MAP_BYTES).unwrap()
}
fn region(data: &[u8], portion: Portion) -> Region<'_> {
    Region {
        map: view(data, 70205),
        row: 32,
        column: 32,
        portion,
    }
}
fn run(data: &[u8], portion: Portion, skip: bool) -> Mesh {
    project(&[region(data, portion)], skip, Limits::default()).unwrap()
}
fn bits(v: [f32; 3]) -> [u32; 3] {
    v.map(f32::to_bits)
}

#[test]
fn flat_terrain_produces_no_solid_surface() {
    let m = run(&fixture(70205, 0, 0, false), Portion::Entire, false);
    assert!(!m.has_triangles());
    assert!(m.solid_vertices().is_empty());
}
#[test]
fn terrain_grid_coordinates_winding_and_signed_zero() {
    let m = run(&fixture(70205, 3, 0, false), Portion::Entire, false);
    assert_eq!(m.solid_vertices.len(), V9_COUNT + V8_COUNT);
    assert_eq!(m.solid_triangles.len(), 65536);
    assert_eq!(
        bits(m.solid_vertices[0]),
        [(-0.0f32).to_bits(), 2f32.to_bits(), (-0.0f32).to_bits()]
    );
    assert_eq!(
        m.solid_triangles[..4],
        [
            [16641, 1, 0],
            [16641, 130, 1],
            [129, 16641, 0],
            [129, 130, 16641]
        ]
    );
    assert_eq!(m.solid_triangles[65535], [16639, 16640, 33024]);
    assert_eq!(
        bits(m.solid_vertices[V9_COUNT]),
        bits([-STEP / 2.0, 2.0, -STEP / 2.0])
    );
}
#[test]
fn integer_packing_and_float_samples_are_reconstructed() {
    for (kind, encoding) in [
        (1, Encoding::Uint8),
        (2, Encoding::Uint16),
        (3, Encoding::Float32),
    ] {
        let raw = fixture(70205, kind, 0, false);
        let v = view(&raw, 70205);
        assert_eq!(v.height().encoding(), encoding);
        let m = run(&raw, Portion::Top, false);
        assert_eq!(m.solid_vertices[0][1], 2.0);
        assert_eq!(
            m.solid_vertices[212][1].to_bits(),
            v.height().max_height_bits()
        );
    }
}
#[test]
fn all_border_portions_select_exact_squares() {
    let data = fixture(70205, 3, 0, false);
    for (portion, first, last) in [
        (Portion::Top, 0, 127),
        (Portion::Right, 127, 16383),
        (Portion::Left, 0, 16256),
        (Portion::Bottom, 16256, 16383),
    ] {
        let m = run(&data, portion, false);
        assert_eq!(m.solid_triangles.len(), 512);
        assert_eq!(m.solid_triangles[0][0], (V9_COUNT + first) as u32);
        assert_eq!(m.solid_triangles[510][1], (V9_COUNT + last) as u32);
    }
}
#[test]
fn holes_remove_only_solid_triangles() {
    let raw = fixture(70205, 3, 0, true);
    let holes = view(&raw, 70205).holes().unwrap();
    let count: usize = holes.iter().map(|b| b.count_ones() as usize).sum();
    let m = run(&raw, Portion::Entire, false);
    assert_eq!(m.solid_triangles.len(), (16384 - count) * 4);
    assert_eq!(m.solid_vertices.len(), 33025);
}
#[test]
fn above_below_equal_liquid_selection() {
    let above = run(&fixture(70205, 1, 1, false), Portion::Entire, false);
    assert!(above.solid_triangles.is_empty());
    assert_eq!(above.liquid_triangles.len(), 32768);
    let below = run(&fixture(70205, 1, 2, false), Portion::Entire, false);
    assert!(below.liquid_triangles.is_empty());
    assert_eq!(below.solid_triangles.len(), 65536);
    let equal = run(&fixture(70205, 3, 3, false), Portion::Entire, false);
    assert!(!equal.liquid_triangles.is_empty());
    assert!(!equal.solid_triangles.is_empty());
}
#[test]
fn flat_liquid_still_contributes_without_terrain() {
    let m = run(&fixture(70205, 0, 1, false), Portion::Entire, false);
    assert_eq!(m.liquid_vertices.len(), 16641);
    assert_eq!(m.liquid_triangles.len(), 32768);
    assert_eq!(m.liquid_triangles[..2], [[130, 1, 0], [129, 130, 0]]);
}
#[test]
fn liquid_classification_dark_and_ignored_bits() {
    for (k, area) in [
        (1, Some(9)),
        (5, None),
        (6, Some(8)),
        (7, Some(8)),
        (8, Some(9)),
        (9, None),
    ] {
        let m = run(&fixture(70205, 0, k, false), Portion::Top, false);
        if let Some(a) = area {
            assert_eq!(m.liquid_areas, vec![a; 256]);
        } else {
            assert!(m.liquid_triangles.is_empty());
        }
    }
    assert!(!run(&fixture(70205, 3, 5, false), Portion::Top, false).has_triangles());
}
#[test]
fn skip_liquids_also_disables_dark_water_exclusion() {
    let m = run(&fixture(70205, 3, 5, false), Portion::Entire, true);
    assert_eq!(m.solid_triangles.len(), 65536);
    assert!(m.liquid_vertices.is_empty());
}
#[test]
fn empty_crop_liquid_has_no_triangles() {
    let m = run(&fixture(70205, 3, 11, false), Portion::Entire, false);
    assert!(m.liquid_triangles.is_empty());
    assert_eq!(m.solid_triangles.len(), 65536);
}
#[test]
fn cropped_liquid_repairs_only_invalid_heights() {
    let m = run(&fixture(70205, 0, 4, false), Portion::Entire, false);
    assert!(!m.liquid_triangles.is_empty());
    assert!(m.liquid_triangles.len() < 100);
    assert!(
        m.liquid_vertices
            .iter()
            .all(|v| v.iter().all(|x| x.is_finite()))
    );
    assert!(m.liquid_vertices.iter().any(|v| v[1] == 5000.0));
}
#[test]
fn repeated_regions_offset_indices_and_keep_distinct_vertices() {
    let raw = fixture(70205, 3, 1, false);
    let r = region(&raw, Portion::Top);
    let m = project(&[r, r], false, Limits::default()).unwrap();
    assert_eq!(m.liquid_triangles[256], [16771, 16642, 16641]);
    let c = m.compact(Limits::default()).unwrap();
    assert_eq!(c.liquid_vertices.len(), 516);
    assert_eq!(bits(c.liquid_vertices[0]), bits(c.liquid_vertices[258]));
}
#[test]
fn compaction_is_first_encounter_not_coordinate_deduplication() {
    let m = run(&fixture(70205, 3, 0, false), Portion::Top, false);
    let first = bits(m.solid_vertices[16641]);
    let c = m.compact(Limits::default()).unwrap();
    assert_eq!(bits(c.solid_vertices[0]), first);
    assert_eq!(c.solid_triangles[0], [0, 1, 2]);
    assert_eq!(c.solid_vertices.len(), 386);
}
#[test]
fn empty_compaction_removes_unused_liquid_and_terrain_vertices() {
    let m = run(&fixture(70205, 3, 5, false), Portion::Entire, false)
        .compact(Limits::default())
        .unwrap();
    assert!(m.solid_vertices.is_empty());
    assert!(m.liquid_vertices.is_empty());
}
#[test]
fn neighborhood_order_and_empty_center_gate() {
    let raw = fixture(70205, 3, 0, false);
    let r = region(&raw, Portion::Entire);
    let n = Neighbors {
        column_next: Some(r.map),
        column_previous: Some(r.map),
        row_next: Some(r.map),
        row_previous: Some(r.map),
    };
    let m = project_neighborhood(r, n, false, Limits::default()).unwrap();
    assert_eq!(m.solid_vertices.len(), 5 * 33025);
    assert_eq!(m.solid_triangles.len(), 65536 + 4 * 512);
    assert_eq!(m.solid_vertices[33025][0], -GRID_SIZE);
    assert_eq!(m.solid_vertices[2 * 33025][0], GRID_SIZE);
    let flat = fixture(70205, 0, 0, false);
    let empty =
        project_neighborhood(region(&flat, Portion::Entire), n, false, Limits::default()).unwrap();
    assert!(!empty.has_triangles());
    assert!(empty.solid_vertices.is_empty());
}
#[test]
fn bounds_build_and_region_count_are_checked() {
    let raw = fixture(70205, 3, 0, false);
    let mut r = region(&raw, Portion::Top);
    assert!(matches!(
        project(&[], false, Limits::default()),
        Err(Error::Limit)
    ));
    assert!(matches!(
        project(&[r; 6], false, Limits::default()),
        Err(Error::Limit)
    ));
    r.row = 64;
    assert!(matches!(
        project(&[r], false, Limits::default()),
        Err(Error::Coordinates)
    ));
    r.row = 0;
    let n = Neighbors {
        row_previous: Some(r.map),
        ..Default::default()
    };
    r.portion = Portion::Entire;
    assert!(matches!(
        project_neighborhood(r, n, false, Limits::default()),
        Err(Error::Coordinates)
    ));
    let other = fixture(70170, 3, 0, false);
    let other = Region {
        map: view(&other, 70170),
        ..r
    };
    assert!(matches!(
        project(&[r, other], false, Limits::default()),
        Err(Error::BuildMismatch)
    ));
}
#[test]
fn policy_caps_reject_before_projection_and_compaction() {
    let raw = fixture(70205, 3, 1, false);
    let r = region(&raw, Portion::Entire);
    for limits in [
        Limits {
            vertices: 1,
            ..Default::default()
        },
        Limits {
            triangles: 1,
            ..Default::default()
        },
        Limits {
            heap_bytes: 1,
            ..Default::default()
        },
        Limits {
            vertices: usize::MAX,
            ..Default::default()
        },
        Limits {
            triangles: usize::MAX,
            ..Default::default()
        },
        Limits {
            heap_bytes: usize::MAX,
            ..Default::default()
        },
    ] {
        assert!(matches!(project(&[r], false, limits), Err(Error::Limit)));
    }
    assert!(matches!(
        run(&raw, Portion::Entire, false).compact(Limits {
            heap_bytes: 1,
            ..Default::default()
        }),
        Err(Error::Limit)
    ));
}
#[test]
fn exact_projection_limits_and_compaction_are_admitted() {
    let raw = fixture(70205, 3, 0, false);
    let cap = (33025 + 512) * 12;
    let m = project(
        &[region(&raw, Portion::Top)],
        false,
        Limits {
            vertices: 33025,
            triangles: 512,
            heap_bytes: cap,
        },
    )
    .unwrap();
    assert_eq!(m.capacity_bytes(), cap);
    let c = m
        .compact(Limits {
            vertices: 33025,
            triangles: 512,
            heap_bytes: cap + 33025 * 16,
        })
        .unwrap();
    assert_eq!(c.solid_vertices.len(), 386);
}

mod corpus;

#[test]
fn finite_header_extrema_with_overflowing_integer_reconstruction_reject() {
    let mut raw = fixture(70205, 1, 0, false);
    let at = u32::from_le_bytes(raw[20..24].try_into().unwrap()) as usize;
    raw[at + 8..at + 12].copy_from_slice(&(-f32::MAX).to_le_bytes());
    raw[at + 12..at + 16].copy_from_slice(&f32::MAX.to_le_bytes());
    let r = region(&raw, Portion::Top); // complete finite wire input passes verification
    assert!(matches!(
        project(&[r], false, Limits::default()),
        Err(Error::NonFinite)
    ));
}

#[test]
fn liquid_partial_average_overflow_rejects_whole_projection() {
    let mut raw = fixture(70205, 0, 4, false);
    let at = u32::from_le_bytes(raw[28..32].try_into().unwrap()) as usize;
    let w = raw[at + 10] as usize;
    assert_eq!(raw[at + 4], liquid_section::NO_TYPE);
    for (index, value) in [(0, -2000.0f32), (1, -3.0e38f32), (w + 1, -3.0e38f32)] {
        raw[at + 16 + index * 4..at + 20 + index * 4].copy_from_slice(&value.to_le_bytes());
    }
    assert!(matches!(
        project(&[region(&raw, Portion::Entire)], false, Limits::default()),
        Err(Error::NonFinite)
    ));
}

#[test]
fn five_full_regions_fit_the_fixed_combined_capacity_ceiling() {
    let raw = fixture(70205, 3, 10, true);
    let m = project(
        &[region(&raw, Portion::Entire); 5],
        false,
        Limits::default(),
    )
    .unwrap();
    assert_eq!(m.solid_vertices.len() + m.liquid_vertices.len(), 248330);
    assert!(m.capacity_bytes() < Limits::default().heap_bytes);
    assert!(
        m.solid_triangles
            .iter()
            .flatten()
            .all(|&i| (i as usize) < m.solid_vertices.len())
    );
    assert!(
        m.liquid_triangles
            .iter()
            .flatten()
            .all(|&i| (i as usize) < m.liquid_vertices.len())
    );
    assert_eq!(m.liquid_triangles.len(), m.liquid_areas.len());
    m.compact(Limits::default()).unwrap();
}
