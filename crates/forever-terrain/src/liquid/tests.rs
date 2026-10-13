// SPDX-License-Identifier: GPL-3.0-or-later
//! Original synthetic bytes only. No filesystem, client or upstream fixtures.
use super::*;
use crate::Limits;
use std::collections::BTreeMap;
fn put16(raw: &mut [u8], at: usize, n: u16) {
    raw[at..at + 2].copy_from_slice(&n.to_le_bytes());
}
fn put32(raw: &mut [u8], at: usize, n: u32) {
    raw[at..at + 4].copy_from_slice(&n.to_le_bytes());
}
fn push64(raw: &mut Vec<u8>, n: u64) {
    raw.extend(n.to_le_bytes());
}
fn frame(tag: [u8; 4], data: &[u8]) -> Vec<u8> {
    let mut bytes: Vec<_> = tag.into_iter().rev().collect();
    bytes.extend((data.len() as u32).to_le_bytes());
    bytes.extend(data);
    bytes
}
fn file(cells: &[Vec<u8>], liquid: Option<&[u8]>) -> Vec<u8> {
    let mut out = frame(*b"MVER", &18u32.to_le_bytes());
    for c in cells {
        out.extend(c);
    }
    if let Some(data) = liquid {
        out.extend(frame(*b"MH2O", data));
    }
    out
}
fn cell(x: u32, y: u32, flags: u32, size: u32, children: &[Vec<u8>]) -> Vec<u8> {
    let mut p = vec![0; 128];
    put32(&mut p, 0, flags);
    put32(&mut p, 4, x);
    put32(&mut p, 8, y);
    put32(&mut p, 100, size);
    for c in children {
        p.extend(c);
    }
    frame(*b"MCNK", &p)
}
fn mclq(flags: u8, height: u32) -> Vec<u8> {
    let mut p = vec![0; 804];
    for i in 0..81 {
        put32(&mut p, 12 + i * 8, height);
    }
    p[656..720].fill(flags);
    p
}
fn old_cell(x: u32, y: u32, flags: u32, tile_flags: u8, height: f32) -> Vec<u8> {
    cell(
        x,
        y,
        flags,
        812,
        &[frame(*b"MCLQ", &mclq(tile_flags, height.to_bits()))],
    )
}
struct Tables {
    types: BTreeMap<u32, LiquidType>,
    materials: BTreeMap<u32, LiquidMaterial>,
}
impl Default for Tables {
    fn default() -> Self {
        Self {
            types: [
                (
                    7,
                    LiquidType {
                        sound_bank: 0,
                        material_id: 3,
                    },
                ),
                (
                    2,
                    LiquidType {
                        sound_bank: 1,
                        material_id: 5,
                    },
                ),
            ]
            .into(),
            materials: [
                (
                    3,
                    LiquidMaterial {
                        flags: 0,
                        vertex_format: 0,
                    },
                ),
                (
                    5,
                    LiquidMaterial {
                        flags: 0,
                        vertex_format: 2,
                    },
                ),
            ]
            .into(),
        }
    }
}
impl LiquidLookup for Tables {
    fn liquid_type(&self, id: u32) -> Option<LiquidType> {
        self.types.get(&id).copied()
    }
    fn material(&self, id: u32) -> Option<LiquidMaterial> {
        self.materials.get(&id).copied()
    }
}
fn project(bytes: &[u8], tables: &Tables, ignore: bool) -> Result<LiquidProjection, Error> {
    let adt = Adt::parse(bytes, Limits::default()).unwrap();
    project_liquids(&adt, tables, ignore)
}
fn values(format: u16, count: usize, base: f32) -> Vec<u8> {
    let mut raw = Vec::new();
    for i in 0..count {
        if format == 4 || format == 5 {
            raw.extend([0xff, 0xee, 0xdd, 0xcc]);
        }
        raw.extend((base + i as f32).to_bits().to_le_bytes());
    }
    raw
}
struct Spec {
    id: u16,
    format: u16,
    rect: [u8; 4],
    used: u32,
    levels: [u32; 2],
    bitmap: Option<u64>,
    attrs: Option<(u64, u64)>,
    vertices: Option<Vec<u8>>,
}
impl Default for Spec {
    fn default() -> Self {
        Self {
            id: 7,
            format: 0,
            rect: [2, 3, 2, 2],
            used: 1,
            levels: [0; 2],
            bitmap: None,
            attrs: None,
            vertices: Some(values(0, 9, 10.0)),
        }
    }
}
struct Mh {
    data: Vec<u8>,
}
impl Mh {
    fn new() -> Self {
        Self {
            data: vec![0; MH2O_HEADER_BYTES],
        }
    }
    fn add(&mut self, x: usize, y: usize, s: Spec) -> usize {
        let header = (y * 16 + x) * 12;
        let at = self.data.len();
        self.data.resize(at + 24, 0);
        put32(&mut self.data, header, at as u32);
        put32(&mut self.data, header + 4, s.used);
        put16(&mut self.data, at, s.id);
        put16(&mut self.data, at + 2, s.format);
        put32(&mut self.data, at + 4, s.levels[0]);
        put32(&mut self.data, at + 8, s.levels[1]);
        self.data[at + 12..at + 16].copy_from_slice(&s.rect);
        if let Some((fish, deep)) = s.attrs {
            let offset = self.data.len();
            put32(&mut self.data, header + 8, offset as u32);
            push64(&mut self.data, fish);
            push64(&mut self.data, deep);
        }
        if let Some(bitmap) = s.bitmap {
            let offset = self.data.len();
            put32(&mut self.data, at + 16, offset as u32);
            push64(&mut self.data, bitmap);
        }
        if let Some(data) = s.vertices {
            let offset = self.data.len();
            put32(&mut self.data, at + 20, offset as u32);
            self.data.extend(data);
        }
        at
    }
}

#[test]
fn mclq_metadata_offsets_borrowing_and_opaque_tail_are_exact() {
    let mut data = mclq(0x0f, (-0.0f32).to_bits());
    put32(&mut data, 0, 0x7fc12345);
    put32(&mut data, 4, 0xff800000);
    put32(&mut data, 8 + (7 * 9 + 3) * 8, 0xaabbccdd);
    put32(&mut data, 12 + (7 * 9 + 3) * 8, 15.5f32.to_bits());
    data[656 + 6 * 8 + 2] = 0x8f;
    data[720..804].fill(0xbb);
    data.extend([1, 2]);
    let view = Mclq::parse(&data).unwrap();
    assert_eq!(view.raw().as_ptr(), data.as_ptr());
    assert_eq!(view.level_bits(), [0x7fc12345, 0xff800000]);
    assert_eq!(view.light(3, 7), Some(0xaabbccdd));
    assert_eq!(view.height_bits(3, 7), Some(15.5f32.to_bits()));
    assert_eq!(view.tile_flags(2, 6), Some(0x8f));
    assert_eq!(view.opaque_tail().len(), 86);
    assert!(view.height_bits(9, 0).is_none());
    assert!(view.light(usize::MAX, 0).is_none());
    assert!(view.tile_flags(0, 8).is_none());
    assert_eq!(Mclq::parse(&data[..803]).err(), Some(Error::Bounds));
}
#[test]
fn empty_projection_has_exact_dimensions_and_zero_initialization() {
    let p = project(&file(&[], None), &Tables::default(), false).unwrap();
    assert_eq!(
        (
            p.show().len(),
            p.entries().len(),
            p.flags().len(),
            p.height_bits().len()
        ),
        (16384, 256, 256, 16641)
    );
    assert!(p.show().iter().all(|&x| x == 0));
    assert!(p.entries().iter().all(|&x| x == 0));
    assert!(p.flags().iter().all(|&x| x == 0));
    assert!(p.height_bits().iter().all(|&x| x == 0));
}
#[test]
fn mclq_flag_byte_equality_type_precedence_and_deep_water_are_exact() {
    let mut raw = mclq(0x0f, 5f32.to_bits());
    raw[656] = 0x8f;
    raw[657] = 0x80;
    raw[658] = 0;
    for ignore in [false, true] {
        let p = project(
            &file(
                &[cell(2, 7, 4 | 8 | 16, 812, &[frame(*b"MCLQ", &raw)])],
                None,
            ),
            &Tables::default(),
            ignore,
        )
        .unwrap();
        assert_eq!(p.entries()[7 * 16 + 2], 3);
        assert_eq!(
            p.flags()[7 * 16 + 2],
            WATER | OCEAN | MAGMA | if ignore { 0 } else { DARK_WATER }
        );
        assert_eq!(&p.show()[56 * 128 + 16..56 * 128 + 20], &[1, 1, 1, 0]);
        assert_eq!(p.height_bits()[64 * 129 + 24], 5f32.to_bits());
        assert_eq!(p.height_bits()[55 * 129 + 16], 0);
    }
}
#[test]
fn mclq_presence_gate_ignores_inactive_or_missing_payloads() {
    for size in [0, 8] {
        let bytes = file(
            &[cell(
                0,
                0,
                4,
                size,
                &[frame(*b"MCLQ", &[0]), frame(*b"MCLQ", &[0])],
            )],
            None,
        );
        assert!(project(&bytes, &Tables::default(), false).is_ok());
    }
    assert!(
        project(
            &file(&[cell(0, 0, 4, 999, &[])], None),
            &Tables::default(),
            false
        )
        .is_ok()
    );
    let bytes = file(&[cell(0, 0, 4, 9, &[frame(*b"MCLQ", &[0])])], None);
    assert_eq!(
        project(&bytes, &Tables::default(), false).err(),
        Some(Error::Bounds)
    );
}
#[test]
fn duplicate_liquid_chunks_fail_only_when_selected() {
    let child = frame(*b"MCLQ", &mclq(0, 0));
    assert_eq!(
        project(
            &file(&[cell(0, 0, 0, 9, &[child.clone(), child])], None),
            &Tables::default(),
            false
        )
        .err(),
        Some(Error::DuplicateChunk)
    );
    let mut bytes = file(&[], Some(&Mh::new().data));
    bytes.extend(frame(*b"MH2O", &Mh::new().data));
    assert_eq!(
        project(&bytes, &Tables::default(), false).err(),
        Some(Error::DuplicateChunk)
    );
}
#[test]
fn mclq_duplicates_accumulate_show_flags_and_overwrite_entry_height() {
    let bytes = file(
        &[
            old_cell(2, 7, 4, 0x80, 10.0),
            old_cell(2, 7, 16, 0x0f, 20.0),
        ],
        None,
    );
    let p = project(&bytes, &Tables::default(), false).unwrap();
    assert_eq!(p.entries()[114], 3);
    assert_eq!(p.flags()[114], WATER | MAGMA | DARK_WATER);
    assert_eq!(p.show()[56 * 128 + 16], 1);
    assert_eq!(p.height_bits()[56 * 129 + 16], 20f32.to_bits());
}
#[test]
fn mclq_shared_edges_follow_physical_order_not_coordinate_order() {
    for reverse in [false, true] {
        let a = old_cell(0, 0, 0, 0x0f, 1.0);
        let b = old_cell(1, 0, 0, 0x0f, 2.0);
        let cells = if reverse { vec![b, a] } else { vec![a, b] };
        let p = project(&file(&cells, None), &Tables::default(), false).unwrap();
        assert_eq!(
            p.height_bits()[3 * 129 + 8],
            if reverse { 1f32 } else { 2f32 }.to_bits()
        );
    }
}
#[test]
fn mh2o_metadata_index_instances_and_raw_fields_are_exact() {
    let mut mh = Mh::new();
    let at = mh.add(
        2,
        7,
        Spec {
            levels: [0x7fc12345, 0xff800000],
            attrs: Some((0x1234, 0x8000000000000000)),
            bitmap: Some(0x1122334455667788),
            ..Spec::default()
        },
    );
    let view = Mh2o::parse(&mh.data).unwrap();
    assert_eq!(view.raw().as_ptr(), mh.data.as_ptr());
    assert!(view.cell(16, 0).is_none());
    assert!(view.cell(0, usize::MAX).is_none());
    let cell = view.cell(2, 7).unwrap();
    assert_eq!(cell.instance_offset(), at as u32);
    assert_eq!(cell.declared_layers(), 1);
    assert_eq!(
        cell.attributes().unwrap(),
        LiquidAttributes {
            fishable: 0x1234,
            deep: 0x8000000000000000
        }
    );
    let h = cell.first_instance().unwrap().unwrap();
    assert_eq!(h.raw().len(), 24);
    assert_eq!(h.liquid_type(), 7);
    assert_eq!(h.vertex_format(), 0);
    assert_eq!(h.level_bits(), [0x7fc12345, 0xff800000]);
    assert_eq!(
        h.raw_rectangle(),
        Rectangle {
            x: 2,
            y: 3,
            width: 2,
            height: 2
        }
    );
    assert_eq!(h.exists_bitmap().unwrap(), 0x1122334455667788);
    assert_eq!(Mh2o::parse(&mh.data[..3071]).err(), Some(Error::Bounds));
}
#[test]
fn mh2o_inactive_cells_do_not_follow_offsets_and_first_layer_only_is_used() {
    let mut mh = Mh::new();
    put32(&mut mh.data, 0, u32::MAX);
    put32(&mut mh.data, 8, u32::MAX);
    let zero = Mh2o::parse(&mh.data).unwrap().cell(0, 0).unwrap();
    assert!(zero.first_instance().unwrap().is_none());
    assert_eq!(
        zero.attributes().unwrap(),
        LiquidAttributes {
            fishable: 0,
            deep: 0
        }
    );
    let at = mh.add(
        2,
        7,
        Spec {
            used: u32::MAX,
            ..Spec::default()
        },
    );
    let cell = Mh2o::parse(&mh.data).unwrap().cell(2, 7).unwrap();
    assert_eq!(
        cell.first_instance().unwrap().unwrap().raw().as_ptr(),
        mh.data[at..].as_ptr()
    );
    assert_eq!(
        cell.attributes().unwrap(),
        LiquidAttributes {
            fishable: u64::MAX,
            deep: u64::MAX
        }
    );
    let p = project(&file(&[], Some(&mh.data)), &Tables::default(), false).unwrap();
    assert_eq!(p.height_bits()[59 * 129 + 18], 10f32.to_bits());
    put32(&mut mh.data, (7 * 16 + 2) * 12, 0);
    assert!(
        project(&file(&[], Some(&mh.data)), &Tables::default(), false)
            .unwrap()
            .entries()
            .iter()
            .all(|&x| x == 0)
    );
}
#[test]
fn all_six_vertex_formats_and_depth_type_override_are_projected() {
    for format in 0..=5 {
        let mut mh = Mh::new();
        mh.add(
            2,
            7,
            Spec {
                format,
                vertices: Some(values(format, 9, 10.0)),
                ..Spec::default()
            },
        );
        let p = project(&file(&[], Some(&mh.data)), &Tables::default(), false).unwrap();
        assert_eq!(p.entries()[114], if format == 2 { 2 } else { 7 });
        assert_eq!(
            p.flags()[114],
            if format == 2 {
                OCEAN | DARK_WATER
            } else {
                WATER
            }
        );
        for y in 0..3 {
            for x in 0..3 {
                assert_eq!(
                    p.height_bits()[(59 + y) * 129 + 18 + x],
                    if format == 2 {
                        0
                    } else {
                        (10.0 + (y * 3 + x) as f32).to_bits()
                    }
                );
            }
        }
    }
}
#[test]
fn mh2o_exists_mask_is_lsb_first_rectangular_and_absent_means_all() {
    for mask in [Some(0b1001), None] {
        let mut mh = Mh::new();
        mh.add(
            2,
            7,
            Spec {
                bitmap: mask,
                ..Spec::default()
            },
        );
        let p = project(&file(&[], Some(&mh.data)), &Tables::default(), false).unwrap();
        assert_eq!(p.show()[59 * 128 + 18], 1);
        assert_eq!(p.show()[60 * 128 + 19], 1);
        assert_eq!(p.show()[59 * 128 + 19], u8::from(mask.is_none()));
        assert_eq!(p.show()[60 * 128 + 18], u8::from(mask.is_none()));
        assert_eq!(p.show()[59 * 128 + 17], 0);
        assert_eq!(p.show()[61 * 128 + 18], 0);
    }
}
#[test]
fn zero_vertex_offset_and_depth_ignore_vertex_extent_and_unused_levels() {
    for format in [0, 1, 2, 3, 4, 5] {
        let mut mh = Mh::new();
        let at = mh.add(
            0,
            0,
            Spec {
                format,
                vertices: None,
                levels: [0x7fc00001, 0x7f800000],
                ..Spec::default()
            },
        );
        if format == 2 {
            put32(&mut mh.data, at + 20, u32::MAX);
        }
        let p = project(&file(&[], Some(&mh.data)), &Tables::default(), false).unwrap();
        assert!(p.height_bits().iter().all(|&x| x == 0));
    }
}
#[test]
fn indirect_formats_normalize_rect_and_resolve_original_type_material() {
    for format in 0..=5 {
        let mut tables = Tables::default();
        tables.materials.get_mut(&3).unwrap().vertex_format = format as i8;
        let mut mh = Mh::new();
        mh.add(
            2,
            7,
            Spec {
                format: 42,
                rect: [255, 254, 253, 252],
                vertices: Some(values(format, 81, 10.0)),
                ..Spec::default()
            },
        );
        let p = project(&file(&[], Some(&mh.data)), &tables, false).unwrap();
        assert_eq!(p.show()[56 * 128 + 16], 1);
        assert_eq!(p.show()[63 * 128 + 23], 1);
        assert_eq!(
            p.height_bits()[64 * 129 + 24],
            if format == 2 { 0 } else { 90f32.to_bits() }
        );
        assert_eq!(p.entries()[114], if format == 2 { 2 } else { 7 });
    }
    let mut tables = Tables::default();
    tables.materials.clear();
    let mut mh = Mh::new();
    mh.add(
        0,
        0,
        Spec {
            id: 2,
            format: u16::MAX,
            rect: [255; 4],
            vertices: None,
            ..Spec::default()
        },
    );
    assert_eq!(
        project(&file(&[], Some(&mh.data)), &tables, false)
            .unwrap()
            .flags()[0],
        OCEAN | DARK_WATER
    );
}
#[test]
fn type_and_visual_filters_write_entry_but_preserve_previous_mclq_projection() {
    for missing in [true, false] {
        let mut tables = Tables::default();
        if missing {
            tables.types.remove(&7);
        } else {
            tables.materials.get_mut(&3).unwrap().flags = MATERIAL_VISUAL_ONLY;
        }
        let mut mh = Mh::new();
        let at = mh.add(
            2,
            7,
            Spec {
                format: 41,
                rect: [255; 4],
                vertices: None,
                ..Spec::default()
            },
        );
        put32(&mut mh.data, at + 16, u32::MAX);
        put32(&mut mh.data, at + 20, u32::MAX);
        put32(&mut mh.data, (7 * 16 + 2) * 12 + 8, u32::MAX);
        let p = project(
            &file(&[old_cell(2, 7, 4, 0, 20.0)], Some(&mh.data)),
            &tables,
            false,
        )
        .unwrap();
        assert_eq!(p.entries()[114], 7);
        assert_eq!(p.flags()[114], WATER);
        assert_eq!(p.show()[56 * 128 + 16], 1);
        assert_eq!(p.height_bits()[56 * 129 + 16], 20f32.to_bits());
    }
}
#[test]
fn missing_material_allows_explicit_format_but_indirect_format_is_unsupported() {
    let mut tables = Tables::default();
    tables.materials.clear();
    let mut mh = Mh::new();
    let at = mh.add(0, 0, Spec::default());
    assert!(project(&file(&[], Some(&mh.data)), &tables, false).is_ok());
    put16(&mut mh.data, at + 2, 42);
    assert_eq!(
        project(&file(&[], Some(&mh.data)), &tables, false).err(),
        Some(Error::UnsupportedVertexFormat)
    );
}
#[test]
fn unknown_vertex_formats_fail_after_filters_including_negative_material_lvf() {
    for f in [6, 41] {
        let mut mh = Mh::new();
        mh.add(
            0,
            0,
            Spec {
                format: f,
                vertices: None,
                ..Spec::default()
            },
        );
        assert_eq!(
            project(&file(&[], Some(&mh.data)), &Tables::default(), false).err(),
            Some(Error::UnsupportedVertexFormat)
        );
    }
    for f in [-128, -1, 6, 127] {
        let mut t = Tables::default();
        t.materials.get_mut(&3).unwrap().vertex_format = f;
        let mut mh = Mh::new();
        mh.add(
            0,
            0,
            Spec {
                format: 42,
                vertices: None,
                ..Spec::default()
            },
        );
        assert_eq!(
            project(&file(&[], Some(&mh.data)), &t, false).err(),
            Some(Error::UnsupportedVertexFormat)
        );
    }
}
#[test]
fn sound_bank_flags_and_deep_attributes_match_cellwide_source_test() {
    for sound in [0, 1, 2, 3, 4, 255] {
        for ignore in [false, true] {
            let mut t = Tables::default();
            t.types.get_mut(&7).unwrap().sound_bank = sound;
            let mut mh = Mh::new();
            mh.add(
                0,
                0,
                Spec {
                    bitmap: Some(1),
                    attrs: Some((0, 1 << 63)),
                    ..Spec::default()
                },
            );
            let p = project(&file(&[], Some(&mh.data)), &t, ignore).unwrap();
            let expected = match sound {
                0 => WATER,
                1 => OCEAN | if ignore { 0 } else { DARK_WATER },
                2 => MAGMA,
                3 => SLIME,
                _ => 0,
            };
            assert_eq!(p.flags()[0], expected);
        }
    }
    let mut t = Tables::default();
    t.types.get_mut(&7).unwrap().sound_bank = 1;
    let mut mh = Mh::new();
    mh.add(
        0,
        0,
        Spec {
            attrs: Some((u64::MAX, 0)),
            ..Spec::default()
        },
    );
    assert_eq!(
        project(&file(&[], Some(&mh.data)), &t, false)
            .unwrap()
            .flags()[0],
        OCEAN
    );
}
#[test]
fn mh2o_runs_after_mclq_regardless_of_top_frame_order_and_does_not_clear() {
    let mut mh = Mh::new();
    mh.add(
        2,
        7,
        Spec {
            bitmap: Some(0),
            vertices: None,
            ..Spec::default()
        },
    );
    let mut data = file(&[], Some(&mh.data));
    data.extend(old_cell(2, 7, 16, 0x80, 20.0));
    let p = project(&data, &Tables::default(), false).unwrap();
    assert_eq!(p.entries()[114], 7);
    assert_eq!(p.flags()[114], MAGMA | WATER | DARK_WATER);
    assert_eq!(p.show()[59 * 128 + 18], 1);
    assert_eq!(p.height_bits()[59 * 129 + 18], 0);
    assert_eq!(p.height_bits()[56 * 129 + 16], 20f32.to_bits());
}
#[test]
fn mh2o_cell_iteration_is_row_major_and_shared_vertices_are_overwritten() {
    let mut mh = Mh::new();
    mh.add(
        1,
        0,
        Spec {
            rect: [0, 0, 8, 8],
            vertices: Some(values(0, 81, 200.0)),
            ..Spec::default()
        },
    );
    mh.add(
        0,
        0,
        Spec {
            rect: [0, 0, 8, 8],
            vertices: Some(values(0, 81, 100.0)),
            ..Spec::default()
        },
    );
    mh.add(
        0,
        1,
        Spec {
            rect: [0, 0, 8, 8],
            vertices: Some(values(0, 81, 300.0)),
            ..Spec::default()
        },
    );
    let p = project(&file(&[], Some(&mh.data)), &Tables::default(), false).unwrap();
    assert_eq!(p.height_bits()[3 * 129 + 8], 227f32.to_bits());
    assert_eq!(p.height_bits()[8 * 129 + 3], 303f32.to_bits());
}
#[test]
fn valid_zero_rectangle_and_last_cell_boundaries_are_admitted() {
    let mut mh = Mh::new();
    mh.add(
        15,
        15,
        Spec {
            rect: [8, 8, 0, 0],
            vertices: Some(values(0, 1, 55.0)),
            ..Spec::default()
        },
    );
    let p = project(&file(&[], Some(&mh.data)), &Tables::default(), false).unwrap();
    assert!(p.show().iter().all(|&b| b == 0));
    assert_eq!(p.flags()[255], WATER);
    assert_eq!(p.height_bits()[HEIGHT_COUNT - 1], 55f32.to_bits());
    for rect in [
        [9, 0, 0, 0],
        [0, 9, 0, 0],
        [7, 0, 2, 1],
        [0, 7, 1, 2],
        [0, 0, 255, 255],
    ] {
        let mut mh = Mh::new();
        mh.add(
            15,
            15,
            Spec {
                rect,
                ..Spec::default()
            },
        );
        assert_eq!(
            project(&file(&[], Some(&mh.data)), &Tables::default(), false).err(),
            Some(Error::InvalidRectangle)
        );
    }
}
#[test]
fn all_reference_spans_are_bounded_inside_the_mh2o_payload() {
    for field in [0, 8, 16, 20] {
        let mut mh = Mh::new();
        let at = mh.add(0, 0, Spec::default());
        let offset = if field == 0 || field == 8 {
            field
        } else {
            at + field
        };
        put32(&mut mh.data, offset, u32::MAX);
        let mut data = file(&[], Some(&mh.data));
        data.extend(frame(*b"XXXX", &vec![0; 4096]));
        assert_eq!(
            project(&data, &Tables::default(), false).err(),
            Some(Error::Bounds)
        );
    }
    let mut mh = Mh::new();
    let at = mh.add(
        0,
        0,
        Spec {
            vertices: None,
            ..Spec::default()
        },
    );
    let end = mh.data.len() as u32;
    put32(&mut mh.data, at + 16, end);
    mh.data.extend([0; 7]);
    assert_eq!(
        project(&file(&[], Some(&mh.data)), &Tables::default(), false).err(),
        Some(Error::Bounds)
    );
}
#[test]
fn height_spans_check_exact_four_or_eight_byte_strides() {
    for format in [0, 1, 3, 4, 5] {
        let mut data = values(format, 9, 0.0);
        data.pop();
        let mut mh = Mh::new();
        mh.add(
            0,
            0,
            Spec {
                format,
                vertices: Some(data),
                ..Spec::default()
            },
        );
        assert_eq!(
            project(&file(&[], Some(&mh.data)), &Tables::default(), false).err(),
            Some(Error::Bounds)
        );
    }
}
#[test]
fn used_height_nonfinite_is_rejected_even_when_tiles_invisible() {
    for bits in [0x7fc00001, 0x7f800000, 0xff800000] {
        let old = cell(0, 0, 0, 9, &[frame(*b"MCLQ", &mclq(0x0f, bits))]);
        assert_eq!(
            project(&file(&[old], None), &Tables::default(), false).err(),
            Some(Error::NonFiniteHeight)
        );
        for format in [0, 1, 3, 4, 5] {
            let mut data = values(format, 9, 0.0);
            let stride = if format <= 3 { 4 } else { 8 };
            put32(
                &mut data,
                8 * stride + if stride == 8 { 4 } else { 0 },
                bits,
            );
            let mut mh = Mh::new();
            mh.add(
                0,
                0,
                Spec {
                    format,
                    bitmap: Some(0),
                    vertices: Some(data),
                    ..Spec::default()
                },
            );
            assert_eq!(
                project(&file(&[], Some(&mh.data)), &Tables::default(), false).err(),
                Some(Error::NonFiniteHeight)
            );
        }
    }
}
#[test]
fn finite_raw_height_bits_survive_without_addition_or_clamp() {
    for bits in [
        0x80000000,
        1,
        0x80000001,
        (-3000.0f32).to_bits(),
        f32::MAX.to_bits(),
    ] {
        let mut raw = Vec::new();
        for _ in 0..9 {
            raw.extend(bits.to_le_bytes());
        }
        let mut mh = Mh::new();
        mh.add(
            0,
            0,
            Spec {
                vertices: Some(raw),
                ..Spec::default()
            },
        );
        let source = file(&[], Some(&mh.data));
        let before = source.clone();
        let p = project(&source, &Tables::default(), false).unwrap();
        assert_eq!(p.height_bits()[3 * 129 + 2], bits);
        assert_eq!(source, before);
    }
}
#[test]
fn deep_water_helper_uses_explicit_row_then_column_order_and_closed_ranges() {
    for y in 0..64 {
        for x in 0..64 {
            assert_eq!(
                ignore_deep_water(0, y, x),
                ((39..=40).contains(&y) && (24..=26).contains(&x))
                    || ((41..=46).contains(&y) && (18..=26).contains(&x))
            );
            assert_eq!(ignore_deep_water(1, y, x), y == 43 && (x == 39 || x == 40));
            assert!(!ignore_deep_water(2, y, x));
        }
    }
    assert!(ignore_deep_water(0, 39, 24));
    assert!(!ignore_deep_water(0, 24, 39));
    assert!(!ignore_deep_water(0, u32::MAX, u32::MAX));
}
#[test]
fn error_reports_are_fixed_and_have_no_payload_or_coordinates() {
    for (e, code) in [
        (Error::Limit, "liquid_policy_limit"),
        (Error::Bounds, "liquid_bounds"),
        (Error::DuplicateChunk, "liquid_duplicate_chunk"),
        (Error::InvalidRectangle, "liquid_invalid_rectangle"),
        (
            Error::UnsupportedVertexFormat,
            "liquid_unsupported_vertex_format",
        ),
        (Error::NonFiniteHeight, "liquid_nonfinite_height"),
    ] {
        assert_eq!(e.code(), code);
        assert_eq!(e.to_string(), code);
    }
}
