// SPDX-License-Identifier: GPL-3.0-or-later
// Authored 2026-10-11: continuous non-flat planar synthetic world fixtures.
use super::*;
use forever_terrain::{height, maps};
fn height(row: u8, column: u8, y: f32, x: f32, offset: f32) -> u32 {
    // Binary-exact slopes keep shared V9 boundaries bit-identical. This is an
    // authored world-space plane, without any client-derived sample values.
    (2.0 + (f32::from(row) * 128.0 + y) / 128.0 + (f32::from(column) * 128.0 + x) / 256.0 + offset)
        .to_bits()
}
fn planar(build: u32, row: u8, column: u8, offset: f32) -> Vec<u8> {
    let v9: Vec<_> = (0..height::V9_COUNT)
        .map(|i| height(row, column, (i / 129) as f32, (i % 129) as f32, offset))
        .collect();
    let v8: Vec<_> = (0..height::V8_COUNT)
        .map(|i| {
            height(
                row,
                column,
                (i / 128) as f32 + 0.5,
                (i % 128) as f32 + 0.5,
                offset,
            )
        })
        .collect();
    let height = height::HeightSection::from_bits(
        &v9,
        &v8,
        height::Options {
            pack_integers: false,
        },
        None,
        height::MAX_SECTION_BYTES,
    )
    .unwrap();
    let area = maps::AreaSection::from_ids(&[42; 256]);
    maps::assemble(
        build,
        maps::MapSections {
            area: &area,
            height: &height,
            liquid: None,
            holes: None,
        },
        maps::MAX_MAP_BYTES,
    )
    .unwrap()
}
#[test]
fn planar_v9_borders_match_exactly_in_both_axes() {
    for i in 0..129 {
        assert_eq!(
            height(31, 31, i as f32, 128., 0.),
            height(31, 32, i as f32, 0., 0.)
        );
        assert_eq!(
            height(31, 31, 128., i as f32, 0.),
            height(32, 31, 0., i as f32, 0.)
        );
    }
}
#[test]
fn complete_planar_maps_are_nonflat_on_both_build_tags() {
    for build in [70170, 70205] {
        let b = planar(build, 32, 31, 0.);
        let v = maps::verify(&b, build, maps::MAX_MAP_BYTES).unwrap();
        assert_eq!(v.height().encoding(), height::Encoding::Float32);
        assert!(v.liquid().is_none());
    }
}
#[test]
fn planar_terrain_has_complete_ground_triangles() {
    let b = planar(70205, 32, 31, 0.);
    let v = maps::verify(&b, 70205, maps::MAX_MAP_BYTES).unwrap();
    let mesh = terrain::project(
        &[terrain::Region {
            map: v,
            row: 32,
            column: 31,
            portion: terrain::Portion::Entire,
        }],
        true,
        Default::default(),
    )
    .unwrap();
    assert_eq!(mesh.solid_vertices().len(), 33025);
    assert_eq!(mesh.solid_triangles().len(), 65536);
    assert!(mesh.liquid_triangles().is_empty());
}
#[test]
#[ignore = "private synthetic output root required in RCE_NAVIGATION_WORLD_TERRAIN_OUTPUT"]
fn write_planar_world_inputs() {
    let out = std::env::var_os("RCE_NAVIGATION_WORLD_TERRAIN_OUTPUT").unwrap();
    let out = Path::new(&out);
    assert!(out.is_absolute() && !out.exists());
    assert!(
        out.parent()
            .unwrap()
            .canonicalize()
            .unwrap()
            .ancestors()
            .all(|p| !p.join(".git").exists())
    );
    fs::create_dir(out).unwrap();
    for (name, build, parent) in [
        ("planar-70205", 70205, false),
        ("planar-70170-small", 70170, false),
        ("planar-ancestor-70205", 70205, true),
    ] {
        let dir = out.join(name);
        fs::create_dir(&dir).unwrap();
        fs::create_dir(dir.join("maps")).unwrap();
        fs::create_dir(dir.join("vmaps")).unwrap();
        for id in 1..=if parent { 2 } else { 1 } {
            let tiles = if id == 1 {
                vec![(31, 31), (31, 32), (32, 31), (32, 32)]
            } else {
                vec![(31, 31), (32, 32)]
            };
            let mut list = vec![b'0'; 4108];
            list[..4].copy_from_slice(b"MAPS");
            list[4..8].copy_from_slice(&10u32.to_le_bytes());
            list[8..12].copy_from_slice(&u32::to_le_bytes(build));
            for &(r, c) in &tiles {
                list[12 + 4095 - (usize::from(r) * 64 + usize::from(c))] = b'1';
                fs::write(
                    dir.join("maps").join(format!("{id:04}_{r:02}_{c:02}.map")),
                    planar(build, r, c, if id == 2 { 0.125 } else { 0. }),
                )
                .unwrap();
            }
            fs::write(dir.join("maps").join(format!("{id:04}.tilelist")), list).unwrap();
        }
    }
}
