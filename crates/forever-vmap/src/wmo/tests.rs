// SPDX-License-Identifier: GPL-3.0-or-later
// Authored 2026-10-11. Independent synthetic fixtures; no client data.
use super::*;

fn put(bytes: &mut [u8], at: usize, value: u32) {
    bytes[at..at + 4].copy_from_slice(&value.to_le_bytes());
}
fn put16(bytes: &mut [u8], at: usize, value: u16) {
    bytes[at..at + 2].copy_from_slice(&value.to_le_bytes());
}
fn frame_bytes(tag: &[u8; 4], payload: &[u8]) -> Vec<u8> {
    let mut bytes: Vec<u8> = tag.iter().rev().copied().collect();
    bytes.extend_from_slice(&(payload.len() as u32).to_le_bytes());
    bytes.extend_from_slice(payload);
    bytes
}
fn words(values: &[u32]) -> Vec<u8> {
    values.iter().flat_map(|v| v.to_le_bytes()).collect()
}
fn shorts(values: &[u16]) -> Vec<u8> {
    values.iter().flat_map(|v| v.to_le_bytes()).collect()
}
fn root_header(groups: u32, flags: u16, lods: u16) -> Vec<u8> {
    let mut h = vec![0; 64];
    put(&mut h, 4, groups);
    put(&mut h, 32, 0x1234_abcd);
    put16(&mut h, 60, flags);
    put16(&mut h, 62, lods);
    h
}
fn root_bytes(ids: &[u32], names: &[u8]) -> Vec<u8> {
    let mut bytes = frame_bytes(b"MOHD", &root_header(ids.len() as u32, 0, 0));
    bytes.extend(frame_bytes(b"GFID", &words(ids)));
    bytes.extend(frame_bytes(b"MOGN", names));
    bytes
}
fn group_header(id: u32) -> Vec<u8> {
    let mut h = vec![0; 68];
    put(&mut h, 56, id);
    put16(&mut h, 64, u16::MAX);
    put16(&mut h, 66, u16::MAX);
    h
}
fn group_bytes(header: &[u8], children: &[u8]) -> Vec<u8> {
    let mut payload = header.to_vec();
    payload.extend_from_slice(children);
    frame_bytes(b"MOGP", &payload)
}
fn vertices() -> Vec<[u32; 3]> {
    (0..6)
        .map(|i| {
            [
                (i as f32).to_bits(),
                (-0.0f32).to_bits(),
                (-(i as f32)).to_bits(),
            ]
        })
        .collect()
}
fn children(flags: &[u16], indices: &[u32], points: &[[u32; 3]], wide: bool) -> Vec<u8> {
    let materials: Vec<u8> = if wide {
        flags
            .iter()
            .flat_map(|f| [f.to_le_bytes(), 513u16.to_le_bytes()].concat())
            .collect()
    } else {
        flags.iter().flat_map(|f| [*f as u8, 37]).collect()
    };
    let mut bytes = frame_bytes(if wide { b"MPY2" } else { b"MOPY" }, &materials);
    let raw_indices = if wide {
        words(indices)
    } else {
        shorts(&indices.iter().map(|v| *v as u16).collect::<Vec<_>>())
    };
    bytes.extend(frame_bytes(
        if wide { b"MOVX" } else { b"MOVI" },
        &raw_indices,
    ));
    bytes.extend(frame_bytes(
        b"MOVT",
        &words(&points.iter().flatten().copied().collect::<Vec<_>>()),
    ));
    bytes
}
fn geometry_fixture(wide: bool) -> Vec<u8> {
    let mut data = children(
        &[0x08, 0, 0x20],
        &[4, 2, 4, 0, 3, 0, 5, 2, 1],
        &vertices(),
        wide,
    );
    let mut batches = [0u8; 48];
    put16(&mut batches, 16, 0x1234);
    put16(&mut batches, 40, u16::MAX);
    data.extend(frame_bytes(b"MOBA", &batches));
    group_bytes(&group_header(77), &data)
}
fn parse_root(bytes: &[u8]) -> Root<'_> {
    Root::parse(bytes, Limits::default()).unwrap_or_else(|e| panic!("{e}"))
}
fn parse_group(bytes: &[u8]) -> Group<'_> {
    Group::parse(bytes, Limits::default()).unwrap_or_else(|e| panic!("{e}"))
}
fn root_error(bytes: &[u8], limits: Limits) -> Error {
    match Root::parse(bytes, limits) {
        Err(e) => e,
        Ok(_) => panic!("invalid root admitted"),
    }
}
fn group_error(bytes: &[u8], limits: Limits) -> Error {
    match Group::parse(bytes, limits) {
        Err(e) => e,
        Ok(_) => panic!("invalid group admitted"),
    }
}
fn project(bytes: &[u8]) -> Geometry {
    let root_bytes = root_bytes(&[5], &[]);
    let root = parse_root(&root_bytes);
    let group = parse_group(bytes);
    let contexts = resolve_contexts(
        &root,
        &[GroupInput {
            file_data_id: 5,
            group: &group,
        }],
        Limits::default(),
    )
    .unwrap_or_else(|e| panic!("{e}"));
    contexts[0]
        .project_geometry(Limits::default())
        .unwrap_or_else(|e| panic!("{e}"))
        .unwrap_or_else(|| panic!("unexpected skip"))
}

#[test]
fn root_preserves_mohd_fields_and_optional_ungated_version() {
    let mut h = root_header(0, 0x8004, 27);
    for (i, value) in [11, 0, 13, 14, 15, 16, 17, 0xff00_1122, 19]
        .into_iter()
        .enumerate()
    {
        put(&mut h, i * 4, value);
    }
    let mut bytes = frame_bytes(b"MOHD", &h);
    bytes.extend(frame_bytes(b"MVER", &u32::MAX.to_le_bytes()));
    bytes.extend(frame_bytes(b"UNKN", &[0xaa, 0xbb, 0xcc]));
    let root = parse_root(&bytes);
    assert_eq!(root.header().textures, 11);
    assert_eq!(root.header().doodad_definitions, 16);
    assert_eq!(root.header().color, 0xff00_1122);
    assert_eq!(root.header().root_wmo_id, 19);
    assert_eq!(root.header().flags, 0x8004);
    assert_eq!(root.declared_lod_count(), 1);
    assert_eq!(root.version(), Some(u32::MAX));
    assert_eq!(root.chunks().last().unwrap().payload, [0xaa, 0xbb, 0xcc]);
    assert_eq!(root.chunk_count(), 3);
}

#[test]
fn detailed_gfid_keeps_duplicate_ids_and_omits_only_zero_in_first_lod() {
    let mut bytes = frame_bytes(b"MOHD", &root_header(3, 0x10, 0));
    bytes.extend(frame_bytes(
        b"GFID",
        &words(&[9, 0, 9, 77, 78, 79, 81, 82, 83]),
    ));
    let root = parse_root(&bytes);
    assert_eq!(root.declared_lod_count(), 3);
    assert_eq!(root.detailed_group_ids().collect::<Vec<_>>(), [9, 9]);
    assert_eq!(root.group_file_data_id(1, 0), Some(77));
    assert_eq!(root.group_file_data_id(0, 1), Some(0));
    assert_eq!(root.group_file_data_id(3, 0), None);
    assert_eq!(root.group_file_data_id(0, 3), None);
}

#[test]
fn explicit_lod_count_and_missing_gfid_do_not_invent_fallback() {
    let mut bytes = frame_bytes(b"MOHD", &root_header(2, 0x10, 2));
    bytes.extend(frame_bytes(b"GFID", &words(&[1, 2, 3, 4])));
    assert_eq!(parse_root(&bytes).declared_lod_count(), 2);
    for wrong in [vec![], words(&[1, 2]), words(&[1, 2, 3, 4, 5])] {
        let mut bad = frame_bytes(b"MOHD", &root_header(2, 0x10, 2));
        bad.extend(frame_bytes(b"GFID", &wrong));
        assert_eq!(root_error(&bad, Limits::default()), Error::Shape);
    }
    let bytes = frame_bytes(b"MOHD", &root_header(2, 0, 0));
    let root = parse_root(&bytes);
    assert!(!root.has_gfid());
    assert_eq!(root.detailed_group_ids().count(), 0);
    assert!(
        resolve_contexts(&root, &[], Limits::default())
            .unwrap_or_else(|e| panic!("{e}"))
            .is_empty()
    );
}

#[test]
fn gfid_before_mohd_is_rejected_instead_of_using_a_later_group_count() {
    for ids in [vec![], words(&[5])] {
        let mut bytes = frame_bytes(b"GFID", &ids);
        bytes.extend(frame_bytes(b"MOHD", &root_header(1, 0, 0)));
        assert_eq!(root_error(&bytes, Limits::default()), Error::Unsupported);
    }
}

#[test]
fn missing_gfid_does_not_calculate_a_large_lod_offset_even_on_32_bits() {
    let bytes = frame_bytes(b"MOHD", &root_header(MAX_GROUPS as u32, 0x10, u16::MAX));
    let root = parse_root(&bytes);
    assert_eq!(
        root.group_file_data_id(usize::from(u16::MAX) - 1, MAX_GROUPS - 1),
        None
    );
    assert_eq!(root.group_file_data_id(usize::MAX, usize::MAX), None);
}

#[test]
fn root_doodads_retain_bytes_and_ids_without_acquisition_claims() {
    let mut bytes = root_bytes(&[], b"\0");
    let mut set = [0u8; 32];
    set[..3].copy_from_slice(b"set");
    put(&mut set, 24, 1);
    set[28..].copy_from_slice(&[1, 2, 3, 4]);
    let mut spawn = [0u8; 40];
    put(&mut spawn, 0, 0xaa00_0001);
    for at in [4, 8, 12, 16, 20, 24, 28, 32] {
        put(&mut spawn, at, (-0.0f32).to_bits());
    }
    put(&mut spawn, 36, 0xff00_1234);
    bytes.extend(frame_bytes(b"MODS", &set));
    bytes.extend(frame_bytes(b"MODD", &spawn));
    bytes.extend(frame_bytes(b"MODI", &words(&[0, u32::MAX])));
    let root = parse_root(&bytes);
    assert_eq!(root.doodad_set_count(), 1);
    let set = root.doodad_set(0).unwrap();
    assert_eq!(&set.name_bytes[..3], b"set");
    assert_eq!(set.padding, [1, 2, 3, 4]);
    let spawn = root.doodad_spawn(0).unwrap();
    assert_eq!((spawn.name_index, spawn.name_flags), (1, 0xaa));
    assert_eq!(spawn.rotation_bits, [0x8000_0000; 4]);
    assert_eq!(root.doodad_file_data_id_count(), 2);
    assert_eq!(root.doodad_file_data_id(1), Some(u32::MAX));
    assert_eq!(root.doodad_file_data_id(2), None);
    assert!(root.doodad_name_bytes().is_none());
    assert!(root.doodad_spawn(1).is_none());
}

#[test]
fn modn_is_nul_terminated_bytes_and_mutually_exclusive_with_modi() {
    let mut bytes = root_bytes(&[], &[]);
    bytes.extend(frame_bytes(b"MODN", b"\xff\0\0a.m2\0"));
    assert_eq!(
        parse_root(&bytes).doodad_name_bytes(),
        Some(b"\xff\0\0a.m2\0".as_slice())
    );
    bytes.extend(frame_bytes(b"MODI", &[]));
    assert_eq!(root_error(&bytes, Limits::default()), Error::Unsupported);
    let mut bad = root_bytes(&[], &[]);
    bad.extend(frame_bytes(b"MODN", b"missing-nul"));
    assert_eq!(root_error(&bad, Limits::default()), Error::Name);
}

#[test]
fn complete_root_framing_headers_and_duplicates_are_strict() {
    assert_eq!(root_error(&[], Limits::default()), Error::MissingHeader);
    for size in [0, 63, 65] {
        assert_eq!(
            root_error(&frame_bytes(b"MOHD", &vec![0; size]), Limits::default()),
            Error::Shape
        );
    }
    let original = root_bytes(&[], &[]);
    for size in [1, 7, original.len() - 1] {
        assert_eq!(
            root_error(&original[..size], Limits::default()),
            Error::Framing
        );
    }
    let mut bad = original.clone();
    bad.push(0);
    assert_eq!(root_error(&bad, Limits::default()), Error::Framing);
    for tag in [b"MOHD", b"GFID", b"MOGN"] {
        let mut bad = original.clone();
        bad.extend(frame_bytes(tag, &[]));
        assert_eq!(root_error(&bad, Limits::default()), Error::Duplicate);
    }
    let mut bad = original;
    bad.extend(frame_bytes(b"MVER", &[0; 3]));
    assert_eq!(root_error(&bad, Limits::default()), Error::Shape);
}

#[test]
fn root_record_shapes_sets_and_nonfinite_are_rejected() {
    for (tag, width) in [(b"MODS", 32), (b"MODD", 40), (b"MODI", 4)] {
        let mut bad = root_bytes(&[], &[]);
        bad.extend(frame_bytes(tag, &vec![0; width - 1]));
        assert_eq!(root_error(&bad, Limits::default()), Error::Shape);
    }
    let mut set = [0; 32];
    put(&mut set, 24, 1);
    let mut bad = root_bytes(&[], &[]);
    bad.extend(frame_bytes(b"MODS", &set));
    assert_eq!(root_error(&bad, Limits::default()), Error::Bounds);
    let mut spawn = [0; 40];
    put(&mut spawn, 28, f32::NAN.to_bits());
    let mut bad = root_bytes(&[], &[]);
    bad.extend(frame_bytes(b"MODD", &spawn));
    assert_eq!(root_error(&bad, Limits::default()), Error::NonFinite);
    let mut h = root_header(0, 0, 0);
    put(&mut h, 36, 1.0f32.to_bits());
    assert_eq!(
        root_error(&frame_bytes(b"MOHD", &h), Limits::default()),
        Error::InvertedBounds
    );
}

#[test]
fn narrow_and_wide_geometry_match_compaction_without_winding_swap() {
    for wide in [false, true] {
        let bytes = geometry_fixture(wide);
        let g = parse_group(&bytes);
        assert_eq!(
            g.triangle_material(0).unwrap().material,
            if wide { 513 } else { 37 }
        );
        assert_eq!(
            (g.vertex_count(), g.triangle_count(), g.index_count()),
            (6, 3, 9)
        );
        let projection = project(&bytes);
        assert_eq!(projection.indices(), [2, 1, 2, 3, 1, 0]);
        assert_eq!(
            projection.vertices_bits(),
            [vertices()[1], vertices()[2], vertices()[4], vertices()[5]]
        );
        assert_eq!(projection.batches(), [0x1234, 65535]);
        assert_eq!(projection.triangle_count(), 2);
        assert_eq!(projection.group_wmo_id(), 77);
        assert_eq!(projection.vertices_bits()[0][1], 0x8000_0000);
    }
}

#[test]
fn all_u16_material_flags_obey_collision_or_render_without_detail() {
    let flags: Vec<u16> = (0..=u16::MAX).collect();
    let indices = vec![0; flags.len() * 3];
    let data = children(&flags, &indices, &[[0; 3]], true);
    let projection = project(&group_bytes(&group_header(0), &data));
    let expected = flags
        .iter()
        .filter(|&&f| (f & 8) != 0 || (f & 0x24) == 0x20)
        .count();
    assert_eq!(projection.triangle_count(), expected);
    assert_eq!(projection.vertices_bits(), [[0; 3]]);
    assert!(projection.indices().iter().all(|i| *i == 0));
}

#[test]
fn wide_indices_retain_vertices_above_u16_domain() {
    let mut points = vec![[0; 3]; 65537];
    points[65536] = [1.0f32.to_bits(), 2.0f32.to_bits(), 3.0f32.to_bits()];
    let data = children(&[8], &[65536, 2, 65536], &points, true);
    let projection = project(&group_bytes(&group_header(0), &data));
    assert_eq!(projection.indices(), [1, 0, 1]);
    assert_eq!(projection.vertices_bits(), [points[2], points[65536]]);
}

#[test]
fn group_header_and_unknown_chunks_are_preserved() {
    let mut h = group_header(0x1234_5678);
    put(&mut h, 4, u32::MAX);
    put(&mut h, 8, 0x200);
    put16(&mut h, 36, 7);
    put16(&mut h, 38, 8);
    put16(&mut h, 40, 9);
    put16(&mut h, 42, 10);
    put(&mut h, 44, 0x1122_3344);
    put(&mut h, 48, 0x0102_0304);
    put(&mut h, 52, u32::MAX);
    put(&mut h, 60, 0x1122_3380);
    put16(&mut h, 64, 0x8000);
    put16(&mut h, 66, 32767);
    let bytes = group_bytes(&h, &frame_bytes(b"UNKN", &[1, 2, 3]));
    let g = parse_group(&bytes);
    assert_eq!(g.header().description_offset, -1);
    assert_eq!((g.header().portal_start, g.header().portal_count), (7, 8));
    assert_eq!(g.header().batches_c, 0x1122_3344);
    assert_eq!(g.header().source_liquid_type, u32::MAX);
    assert_eq!(g.header().parent_or_first_child_split_group, i16::MIN);
    assert_eq!(g.header().next_split_child_group, i16::MAX);
    assert_eq!(g.chunks().next().unwrap().payload, [1, 2, 3]);
    assert_eq!(g.chunk_count(), 2);
}

#[test]
fn empty_and_completely_filtered_geometry_remain_successful_empty_projection() {
    let empty = group_bytes(&group_header(0), &[]);
    let projection = project(&empty);
    assert!(projection.vertices_bits().is_empty());
    assert!(projection.indices().is_empty());
    let data = children(&[0x24], &[0, 1, 2], &vertices(), false);
    let projection = project(&group_bytes(&group_header(0), &data));
    assert_eq!(projection.triangle_count(), 0);
    assert!(projection.vertices_bits().is_empty());
}

#[test]
fn group_exact_container_and_child_boundaries_never_return_prefix() {
    assert_eq!(group_error(&[], Limits::default()), Error::MissingHeader);
    assert_eq!(
        group_error(&frame_bytes(b"MOGP", &[0; 67]), Limits::default()),
        Error::Shape
    );
    let original = geometry_fixture(false);
    let mut bad = original.clone();
    put(&mut bad, 4, 68); // Original loader ignores this; bounded parser does not.
    assert_eq!(group_error(&bad, Limits::default()), Error::Unsupported);
    let mut bad = original.clone();
    put(&mut bad, 4, u32::MAX);
    assert_eq!(group_error(&bad, Limits::default()), Error::Framing);
    let mut bad = group_header(0);
    bad.push(0);
    assert_eq!(
        group_error(&frame_bytes(b"MOGP", &bad), Limits::default()),
        Error::Framing
    );
    let mut bad = original.clone();
    bad.extend(frame_bytes(b"MOGP", &group_header(1)));
    assert_eq!(group_error(&bad, Limits::default()), Error::Duplicate);
    assert_eq!(
        group_error(&original[..original.len() - 1], Limits::default()),
        Error::Framing
    );
}

#[test]
fn duplicate_and_alternative_consumed_arrays_are_not_last_wins() {
    for tag in [b"MOPY", b"MOVI", b"MOVT"] {
        let mut data = children(&[8], &[0, 1, 2], &vertices(), false);
        data.extend(frame_bytes(tag, &[]));
        assert_eq!(
            group_error(&group_bytes(&group_header(0), &data), Limits::default()),
            Error::Duplicate
        );
    }
    for tag in [b"MPY2", b"MOVX"] {
        let mut data = children(&[8], &[0, 1, 2], &vertices(), false);
        data.extend(frame_bytes(tag, &[]));
        assert_eq!(
            group_error(&group_bytes(&group_header(0), &data), Limits::default()),
            Error::Unsupported
        );
    }
    for tag in [b"MOGP", b"MVER"] {
        assert_eq!(
            group_error(
                &group_bytes(&group_header(0), &frame_bytes(tag, &[])),
                Limits::default()
            ),
            Error::Unsupported
        );
    }
}

#[test]
fn all_used_array_shapes_are_exact_and_indices_are_checked_before_filtering() {
    for (tag, width) in [
        (b"MOPY", 2),
        (b"MPY2", 4),
        (b"MOVI", 2),
        (b"MOVX", 4),
        (b"MOVT", 12),
        (b"MOBA", 24),
        (b"MODR", 2),
    ] {
        let bad = group_bytes(&group_header(0), &frame_bytes(tag, &vec![0; width - 1]));
        assert_eq!(group_error(&bad, Limits::default()), Error::Shape);
    }
    let data = children(&[0], &[0, 1, 6], &vertices(), false);
    assert_eq!(
        group_error(&group_bytes(&group_header(0), &data), Limits::default()),
        Error::Index
    );
    let data = children(&[8], &[0, 1], &vertices(), true);
    assert_eq!(
        group_error(&group_bytes(&group_header(0), &data), Limits::default()),
        Error::Shape
    );
}

#[test]
fn nonfinite_vertices_and_unordered_bounds_fail_even_for_unused_geometry() {
    let mut points = vertices();
    points[5][0] = f32::INFINITY.to_bits();
    let data = children(&[0], &[0, 1, 2], &points, false);
    assert_eq!(
        group_error(&group_bytes(&group_header(0), &data), Limits::default()),
        Error::NonFinite
    );
    let mut h = group_header(0);
    put(&mut h, 12, 1.0f32.to_bits());
    assert_eq!(
        group_error(&group_bytes(&h, &[]), Limits::default()),
        Error::InvertedBounds
    );
}

fn liquid() -> Vec<u8> {
    let mut bytes = vec![0; 30];
    for at in [0, 4, 8, 12] {
        put(&mut bytes, at, 1);
    }
    put(&mut bytes, 16, (-0.0f32).to_bits());
    put16(&mut bytes, 28, 0xffff);
    bytes.extend(shorts(&[0x1234, 0x5678]));
    bytes.extend((-0.0f32).to_bits().to_le_bytes());
    bytes.push(0x8f);
    bytes
}

#[test]
fn complete_liquid_view_and_doodad_references_survive_geometry_projection() {
    let raw = liquid();
    let mut data = frame_bytes(b"MLIQ", &raw);
    data.extend(frame_bytes(b"MODR", &shorts(&[0, 65535, 0])));
    let bytes = group_bytes(&group_header(0), &data);
    let g = parse_group(&bytes);
    let l = g.liquid().unwrap();
    assert_eq!(l.bytes(), raw);
    assert_eq!(l.dimensions(), [1; 4]);
    assert_eq!(l.position_bits(), [0x8000_0000, 0, 0]);
    assert_eq!(l.material(), -1);
    assert_eq!(l.vertex(0), Some((0x1234, 0x5678, 0x8000_0000)));
    assert_eq!(l.vertex(1), None);
    assert_eq!(l.tiles(), [0x8f]);
    assert_eq!(g.doodad_reference_count(), 3);
    assert_eq!(g.doodad_reference(1), Some(65535));
    assert_eq!(g.doodad_reference(3), None);
    assert_eq!(project(&bytes).triangle_count(), 0);
}

#[test]
fn liquid_zero_dimensions_are_preserved_without_invented_grid_relation() {
    let raw = [0u8; 30];
    let bytes = group_bytes(&group_header(0), &frame_bytes(b"MLIQ", &raw));
    let g = parse_group(&bytes);
    assert_eq!(g.liquid().unwrap().vertex_count(), 0);
    assert_eq!(g.liquid().unwrap().tiles(), []);
}

#[test]
fn liquid_extents_signed_dimensions_and_all_heights_are_checked() {
    for raw in [liquid()[..38].to_vec(), {
        let mut v = liquid();
        v.push(0);
        v
    }] {
        let bytes = group_bytes(&group_header(0), &frame_bytes(b"MLIQ", &raw));
        assert_eq!(group_error(&bytes, Limits::default()), Error::Shape);
    }
    for (at, bits, expected) in [
        (0, u32::MAX, Error::Shape),
        (0, i32::MAX as u32, Error::Limit),
        (16, f32::NAN.to_bits(), Error::NonFinite),
        (34, f32::NEG_INFINITY.to_bits(), Error::NonFinite),
    ] {
        let mut raw = liquid();
        put(&mut raw, at, bits);
        let bytes = group_bytes(&group_header(0), &frame_bytes(b"MLIQ", &raw));
        assert_eq!(group_error(&bytes, Limits::default()), expected);
    }
}

#[test]
fn group_skips_respect_native_order_and_exact_name_case() {
    let rb = root_bytes(&[1, 2, 3, 4], b"antiportal\0Antiportal\0");
    let root = parse_root(&rb);
    let mut headers: Vec<_> = (0..4).map(group_header).collect();
    put(&mut headers[0], 8, 0x0400_0080);
    put(&mut headers[0], 0, u32::MAX);
    put(&mut headers[1], 8, 0x0400_0000);
    put(&mut headers[1], 0, u32::MAX);
    put(&mut headers[3], 0, 11);
    let files: Vec<_> = headers.iter().map(|h| group_bytes(h, &[])).collect();
    let groups: Vec<_> = files.iter().map(|b| parse_group(b)).collect();
    let input: Vec<_> = groups
        .iter()
        .enumerate()
        .map(|(i, group)| GroupInput {
            file_data_id: i as u32 + 1,
            group,
        })
        .collect();
    let contexts =
        resolve_contexts(&root, &input, Limits::default()).unwrap_or_else(|e| panic!("{e}"));
    assert_eq!(
        contexts
            .iter()
            .map(GroupContext::skip_reason)
            .collect::<Vec<_>>(),
        [
            Some(SkipReason::Unreachable),
            Some(SkipReason::AntiportalFlag),
            Some(SkipReason::AntiportalName),
            None
        ]
    );
    for ctx in &contexts[..3] {
        assert!(
            ctx.project_geometry(Limits::default())
                .unwrap_or_else(|e| panic!("{e}"))
                .is_none()
        );
    }
}

#[test]
fn names_fail_closed_only_when_native_would_dereference_them() {
    for (name, offset, expected) in [
        (b"x".as_slice(), 0, Some(Error::Name)),
        (b"".as_slice(), u32::MAX, Some(Error::Name)),
        (b"x".as_slice(), 1, None),
    ] {
        let rb = root_bytes(&[1], name);
        let root = parse_root(&rb);
        let mut h = group_header(0);
        put(&mut h, 0, offset);
        let gb = group_bytes(&h, &[]);
        let group = parse_group(&gb);
        let result = resolve_contexts(
            &root,
            &[GroupInput {
                file_data_id: 1,
                group: &group,
            }],
            Limits::default(),
        );
        assert_eq!(result.err(), expected);
    }
}

#[test]
fn inventory_checks_order_zeros_duplicates_and_exact_count() {
    let rb = root_bytes(&[1, 0, 1, 2], &[]);
    let root = parse_root(&rb);
    let gb = group_bytes(&group_header(5), &[]);
    let group = parse_group(&gb);
    for ids in [vec![], vec![1, 1], vec![1, 2, 1], vec![1, 0, 1, 2]] {
        let input: Vec<_> = ids
            .into_iter()
            .map(|file_data_id| GroupInput {
                file_data_id,
                group: &group,
            })
            .collect();
        assert_eq!(
            resolve_contexts(&root, &input, Limits::default()).err(),
            Some(Error::Inventory)
        );
    }
    let input: Vec<_> = [1, 1, 2]
        .into_iter()
        .map(|file_data_id| GroupInput {
            file_data_id,
            group: &group,
        })
        .collect();
    let ctx = resolve_contexts(&root, &input, Limits::default()).unwrap_or_else(|e| panic!("{e}"));
    assert_eq!(
        ctx.iter().map(GroupContext::ordinal).collect::<Vec<_>>(),
        [0, 1, 2]
    );
}

#[test]
fn split_ids_use_mutable_single_forward_pass_not_recursive_resolution() {
    let rb = root_bytes(&[1, 2, 3, 4, 5, 6, 7], &[]);
    let root = parse_root(&rb);
    let specs = [
        (10, 2, 0),
        (20, 0, 0),
        (30, 1, 0),
        (40, 0, 0x80),
        (50, 3, 0),
        (60, -1, 0),
        (70, 99, 0),
    ];
    let files: Vec<_> = specs
        .iter()
        .map(|&(id, parent, flags)| {
            let mut h = group_header(id);
            put(&mut h, 8, flags);
            put(&mut h, 60, 0x80);
            put16(&mut h, 64, parent as u16);
            group_bytes(&h, &[])
        })
        .collect();
    let groups: Vec<_> = files.iter().map(|b| parse_group(b)).collect();
    let input: Vec<_> = groups
        .iter()
        .enumerate()
        .map(|(i, group)| GroupInput {
            file_data_id: i as u32 + 1,
            group,
        })
        .collect();
    let ctx = resolve_contexts(&root, &input, Limits::default()).unwrap_or_else(|e| panic!("{e}"));
    assert_eq!(
        ctx.iter()
            .map(GroupContext::group_wmo_id)
            .collect::<Vec<_>>(),
        [30, 30, 30, 40, 40, 60, 70]
    );
}

#[test]
fn limits_reject_raised_ceiling_and_reduced_input_counts_chunks() {
    let rb = root_bytes(&[1], &[]);
    let gb = geometry_fixture(false);
    assert_eq!(
        root_error(
            &rb,
            Limits {
                max_input_bytes: MAX_INPUT_BYTES + 1,
                ..Limits::default()
            }
        ),
        Error::InvalidLimits
    );
    assert_eq!(
        root_error(
            &rb,
            Limits {
                max_input_bytes: rb.len() - 1,
                ..Limits::default()
            }
        ),
        Error::Limit
    );
    assert_eq!(
        root_error(
            &rb,
            Limits {
                max_groups: 0,
                ..Limits::default()
            }
        ),
        Error::Limit
    );
    assert_eq!(
        root_error(
            &rb,
            Limits {
                max_chunks: 2,
                ..Limits::default()
            }
        ),
        Error::Limit
    );
    assert_eq!(
        group_error(
            &gb,
            Limits {
                max_vertices: 5,
                ..Limits::default()
            }
        ),
        Error::Limit
    );
    assert_eq!(
        group_error(
            &gb,
            Limits {
                max_indices: 8,
                ..Limits::default()
            }
        ),
        Error::Limit
    );
    assert_eq!(
        group_error(
            &gb,
            Limits {
                max_metadata_records: 1,
                ..Limits::default()
            }
        ),
        Error::Limit
    );
    assert_eq!(
        group_error(
            &gb,
            Limits {
                max_chunks: 4,
                ..Limits::default()
            }
        ),
        Error::Limit
    );
}

#[test]
fn contexts_and_projection_enforce_reduced_allocation_budget() {
    let rb = root_bytes(&[1], &[]);
    let root = parse_root(&rb);
    let gb = geometry_fixture(false);
    let group = parse_group(&gb);
    let input = [GroupInput {
        file_data_id: 1,
        group: &group,
    }];
    assert_eq!(
        resolve_contexts(
            &root,
            &input,
            Limits {
                max_projection_bytes: 0,
                ..Limits::default()
            }
        )
        .err(),
        Some(Error::Limit)
    );
    let ctx = resolve_contexts(&root, &input, Limits::default()).unwrap_or_else(|e| panic!("{e}"));
    let mut limits = Limits {
        max_projection_bytes: 0,
        ..Limits::default()
    };
    assert_eq!(ctx[0].project_geometry(limits).err(), Some(Error::Limit));
    limits.max_projection_bytes = 24 + 48 + 24 + 8; // renumber + 4 vertices + 6 indices + 2 batches
    let projection = ctx[0]
        .project_geometry(limits)
        .unwrap_or_else(|e| panic!("{e}"));
    assert!(projection.is_some());
    limits.max_projection_bytes -= 1;
    assert_eq!(ctx[0].project_geometry(limits).err(), Some(Error::Limit));
    assert_eq!(
        ctx[0]
            .project_geometry(Limits {
                max_vertices: 5,
                ..Limits::default()
            })
            .err(),
        Some(Error::Limit)
    );
}
