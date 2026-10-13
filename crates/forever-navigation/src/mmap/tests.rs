// SPDX-License-Identifier: GPL-3.0-or-later
// Authored 2026-10-11: synthetic Forever container contracts.
use super::*;
mod corpus;
fn parameters() -> Params {
    Params {
        origin: [-0.0, f32::MIN_POSITIVE, -533.3333],
        tile_width: 533.3333,
        tile_height: 533.3333,
        max_tiles: 4096,
        max_polys: i32::MIN,
    }
}
fn record() -> offmesh::Source {
    offmesh::Source {
        map_id: 7,
        row: 5,
        column: 6,
        from: [1.0, 2.0, 3.0],
        to: [4.0, 5.0, 6.0],
        radius: 0.75,
        connection_flags: 255,
        area: 254,
        flags: 0xfedc,
    }
}
fn global(s: &[offmesh::Source]) -> Vec<u8> {
    encode_global(7, parameters(), s, Limits::default()).unwrap()
}
// Framing-only fixture, deliberately not valid runtime geometry.
fn payload() -> Vec<u8> {
    let mut b = vec![0; 144];
    for (a, v) in [(0, detour::MAGIC), (4, 7), (24, 1), (28, 1)] {
        put(&mut b, a, v);
    }
    b
}
fn tile() -> Vec<u8> {
    encode_payload(&payload(), true, Limits::default()).unwrap()
}

#[test]
fn empty_global_is_exact_forty_bytes_with_native_signed_sentinel() {
    let b = global(&[]);
    assert_eq!(b.len(), 40);
    assert_eq!(word(&b, 0), MAGIC);
    assert_eq!(word(&b, 4), 16);
    assert_eq!(word(&b, 8), (-0.0f32).to_bits());
    assert_eq!(word(&b, 32), 0x80000000);
    assert_eq!(word(&b, 36), 0);
    let v = GlobalView::parse(&b, 7, Limits::default()).unwrap();
    assert_eq!(v.params(), parameters());
    assert_eq!(v.sources().len(), 0);
    assert_eq!(v.bytes(), b);
}
#[test]
fn map_filter_preserves_order_duplicates_and_every_record_field() {
    let mut a = record();
    let mut other = a;
    other.map_id = 8;
    a.radius = -0.0;
    let b = global(&[a, other, record(), a]);
    assert_eq!(b.len(), 172);
    assert_eq!(word(&b, 36), 3);
    let v = GlobalView::parse(&b, 7, Limits::default()).unwrap();
    let s = v.sources().collect::<Vec<_>>();
    assert_eq!(s.len(), 3);
    assert_eq!(s[0].radius.to_bits(), (-0.0f32).to_bits());
    assert_eq!(s[1].radius, 0.75);
    assert_eq!(s[2].row, 5);
    assert_eq!(s[2].column, 6);
    assert_eq!(s[2].from, [1.0, 2.0, 3.0]);
    assert_eq!(s[2].to, [4.0, 5.0, 6.0]);
    assert_eq!(s[2].connection_flags, 255);
    assert_eq!(s[2].area, 254);
    assert_eq!(s[2].flags, 0xfedc);
    assert_eq!(global(&s), b);
}
#[test]
fn raw_global_offmesh_bits_are_not_projected_or_silently_sanitized() {
    let mut a = record();
    a.radius = -4.0;
    a.from[0] = f32::from_bits(0x7fc01234);
    a.row = u32::MAX;
    let b = global(&[a]);
    let v = GlobalView::parse(&b, 7, Limits::default()).unwrap();
    let s = v.sources().next().unwrap();
    assert_eq!(s.from[0].to_bits(), 0x7fc01234);
    assert_eq!(s.radius, -4.0);
    assert_eq!(s.row, u32::MAX);
    assert_eq!(global(&[s]), b);
}
#[test]
fn decoded_records_feed_existing_selection_in_native_axes() {
    let b = global(&[record()]);
    let v = GlobalView::parse(&b, 7, Limits::default()).unwrap();
    let sources = v.sources().collect::<Vec<_>>();
    let s = offmesh::select(
        offmesh::Target {
            map_id: 7,
            row: 5,
            column: 6,
        },
        &sources,
        offmesh::Limits::default(),
    )
    .unwrap();
    assert_eq!(
        s.values()[0].endpoints(),
        [[2.0, 3.0, 1.0], [5.0, 6.0, 4.0]]
    );
    assert_eq!(s.values()[0].direction(), 1);
}
#[test]
fn global_rejects_every_truncation_and_trailing_byte() {
    let b = global(&[record()]);
    for n in 0..b.len() {
        assert!(
            GlobalView::parse(&b[..n], 7, Limits::default()).is_err(),
            "{n}"
        );
    }
    let mut extra = b.clone();
    extra.push(0);
    assert_eq!(
        GlobalView::parse(&extra, 7, Limits::default()).unwrap_err(),
        Error::Size
    );
}
#[test]
fn global_rejects_magic_versions_wrong_map_and_count() {
    for (at, v, error) in [
        (0, 0, Error::Magic),
        (4, 15, Error::Version),
        (36, 0, Error::Size),
        (36, 65537, Error::Limit),
        (40, 8, Error::Map),
    ] {
        let mut b = global(&[record()]);
        put(&mut b, at, v);
        assert_eq!(
            GlobalView::parse(&b, 7, Limits::default()).unwrap_err(),
            error
        );
    }
}
#[test]
fn params_admission_preserves_max_poly_sentinel() {
    for (at, v) in [
        (8, f32::NAN.to_bits()),
        (20, 0),
        (24, (-1.0f32).to_bits()),
        (28, 0),
        (28, (1 << 21) + 1),
        (32, 0),
        (32, 0xffffffff),
    ] {
        let mut b = global(&[]);
        put(&mut b, at, v);
        assert_eq!(
            GlobalView::parse(&b, 7, Limits::default()).unwrap_err(),
            Error::Params
        );
        assert_eq!(
            encode_global(7, params(&b), &[], Limits::default()).unwrap_err(),
            Error::Params
        );
    }
    let mut p = parameters();
    p.max_tiles = 1 << 21;
    p.max_polys = i32::MAX;
    assert!(p.validate().is_ok());
}
#[test]
fn global_limits_include_unselected_inputs_and_output_capacity() {
    let mut other = record();
    other.map_id = 8;
    assert_eq!(
        encode_global(
            7,
            parameters(),
            &[other],
            Limits {
                records: 0,
                ..Limits::default()
            }
        )
        .unwrap_err(),
        Error::Limit
    );
    for l in [
        Limits {
            file_bytes: 39,
            ..Limits::default()
        },
        Limits {
            heap_bytes: 39,
            ..Limits::default()
        },
        Limits {
            records: 65537,
            ..Limits::default()
        },
    ] {
        assert_eq!(
            encode_global(7, parameters(), &[], l).unwrap_err(),
            Error::Limit
        );
    }
    let b = global(&[record()]);
    assert_eq!(
        GlobalView::parse(
            &b,
            7,
            Limits {
                records: 0,
                ..Limits::default()
            }
        )
        .unwrap_err(),
        Error::Limit
    );
    assert!(
        GlobalView::parse(
            &b,
            7,
            Limits {
                heap_bytes: 0,
                ..Limits::default()
            }
        )
        .is_ok()
    );
}
#[test]
fn full_record_ceiling_roundtrips_without_native_struct_padding() {
    let s = vec![record(); 65536];
    let b = global(&s);
    assert_eq!(b.len(), 40 + 65536 * 44);
    assert_eq!(
        GlobalView::parse(&b, 7, Limits::default())
            .unwrap()
            .sources()
            .len(),
        65536
    );
}
#[test]
fn canonical_tile_header_keeps_payload_and_liquid_option() {
    for liquid in [false, true] {
        let b = encode_payload(&payload(), liquid, Limits::default()).unwrap();
        assert_eq!(
            b[..16],
            [MAGIC, 7, 16, 144]
                .into_iter()
                .flat_map(u32::to_le_bytes)
                .collect::<Vec<_>>()
        );
        assert_eq!(b[16..20], [u8::from(liquid), 0, 0, 0]);
        let v = TileView::parse(&b, Limits::default()).unwrap();
        assert_eq!(v.payload(), payload());
        assert_eq!(v.bytes(), b);
        assert_eq!(v.uses_liquids(), liquid);
    }
}
#[test]
fn tile_rejects_every_truncation_and_trailing_byte() {
    let b = tile();
    for n in 0..b.len() {
        assert!(TileView::parse(&b[..n], Limits::default()).is_err(), "{n}");
    }
    let mut b = b;
    b.push(0);
    assert_eq!(
        TileView::parse(&b, Limits::default()).unwrap_err(),
        Error::Size
    );
}
#[test]
fn tile_rejects_versions_padding_liquid_byte_and_size() {
    for (at, v, e) in [
        (0, 0, Error::Magic),
        (4, 6, Error::DetourVersion),
        (8, 15, Error::Version),
        (12, 0, Error::Size),
        (20, 0, Error::Payload),
        (24, 6, Error::DetourVersion),
        (44, 2, Error::Payload),
        (48, u32::MAX, Error::Payload),
    ] {
        let mut b = tile();
        put(&mut b, at, v);
        assert_eq!(TileView::parse(&b, Limits::default()).unwrap_err(), e);
    }
    for (at, v) in [(16, 2), (17, 1), (18, 1), (19, 1)] {
        let mut b = tile();
        b[at] = v;
        assert_eq!(
            TileView::parse(&b, Limits::default()).unwrap_err(),
            Error::Padding
        );
    }
}
#[test]
fn payload_negative_counts_empty_geometry_and_block_mismatch_reject() {
    for at in [24, 28, 32, 36, 40, 44, 48, 52] {
        let mut b = payload();
        put(&mut b, at, u32::MAX);
        assert_eq!(
            encode_payload(&b, true, Limits::default()).unwrap_err(),
            Error::Payload
        );
    }
    let mut b = payload();
    put(&mut b, 24, 0);
    assert_eq!(
        encode_payload(&b, true, Limits::default()).unwrap_err(),
        Error::Payload
    );
}
#[test]
fn tile_limits_charge_header_and_preserve_borrowed_reader() {
    assert_eq!(
        encode_payload(
            &payload(),
            true,
            Limits {
                file_bytes: 163,
                ..Limits::default()
            }
        )
        .unwrap_err(),
        Error::Limit
    );
    assert_eq!(
        encode_payload(
            &payload(),
            true,
            Limits {
                heap_bytes: 163,
                ..Limits::default()
            }
        )
        .unwrap_err(),
        Error::Limit
    );
    let b = tile();
    assert!(
        TileView::parse(
            &b,
            Limits {
                heap_bytes: 0,
                ..Limits::default()
            }
        )
        .is_ok()
    );
    assert_eq!(
        TileView::parse(
            &b,
            Limits {
                file_bytes: 163,
                ..Limits::default()
            }
        )
        .unwrap_err(),
        Error::Limit
    );
}
#[test]
fn filenames_match_forever_separators_and_minimum_widths() {
    assert_eq!(global_path(1), "mmaps/0001.mmap");
    assert_eq!(global_path(u32::MAX), "mmaps/4294967295.mmap");
    assert_eq!(tile_path(7, 3, 63).unwrap(), "mmaps/0007_03_63.mmtile");
    assert_eq!(tile_path(10000, 0, 0).unwrap(), "mmaps/10000_00_00.mmtile");
    assert_eq!(tile_path(0, 64, 0), Err(Error::Coordinates));
    assert_eq!(tile_path(0, 0, 64), Err(Error::Coordinates));
}

#[test]
fn complete_public_pipeline_reaches_versioned_container() {
    use crate::recast::{
        Agent, ContourOptions, DetailOptions, Grid, Heightfield, RegionOptions, Span, SpanInput,
    };
    let spans = (0..8)
        .flat_map(|z| {
            (0..8).map(move |x| SpanInput {
                x,
                z,
                span: Span {
                    min: 0,
                    max: 1,
                    area: 11,
                },
            })
        })
        .collect::<Vec<_>>();
    let m = Heightfield::new(
        Grid {
            width: 8,
            height: 8,
            min: [0.0; 3],
            max: [8.0, 20.0, 8.0],
            cell_size: 1.0,
            cell_height: 0.5,
        },
        crate::recast::Limits::default(),
    )
    .unwrap()
    .add_spans(&spans, 0)
    .unwrap()
    .compact(Agent {
        height: 1,
        climb: 1,
    })
    .unwrap()
    .build_distance_field()
    .unwrap()
    .build_regions(RegionOptions {
        border_size: 1,
        minimum_area: 0,
        merge_area: 0,
    })
    .unwrap()
    .build_contours(ContourOptions::default())
    .unwrap()
    .build_mesh(6)
    .unwrap()
    .build_detail(DetailOptions {
        sample_distance: 16.0,
        maximum_error: 0.5,
    })
    .unwrap()
    .into_tile_mesh();
    let m = crate::recast::TileMesh::merge(vec![m], crate::recast::MergeLimits::default()).unwrap();
    let h = m.header();
    let data = detour::build(
        &m,
        &[],
        detour::Options {
            tile: [0; 3],
            user_id: 0,
            min: h.min,
            max: h.max,
            walkable_height: 2.0,
            walkable_radius: 0.5,
            walkable_climb: 0.5,
            build_bv_tree: true,
            flags: detour::FlagPolicy::Forever,
        },
        detour::Limits::default(),
    )
    .unwrap();
    let bytes = encode_tile(&data, true, Limits::default()).unwrap();
    let tile = TileView::parse(&bytes, Limits::default()).unwrap();
    assert_eq!(tile.payload(), data.bytes());
}
