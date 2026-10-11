// SPDX-License-Identifier: GPL-3.0-or-later
// Authored 2026-10-11: independent synthetic WMO source/raw fixtures.
use super::*;
use std::cell::Cell;

struct Lookup {
    queries: Cell<usize>,
    ignored: Option<u32>,
}
impl Lookup {
    fn empty() -> Self {
        Self {
            queries: Cell::new(0),
            ignored: None,
        }
    }
}
impl LiquidLookup for Lookup {
    fn material_id(&self, type_id: u32) -> Option<u8> {
        self.queries.set(self.queries.get() + 1);
        (self.ignored == Some(type_id)).then_some(7)
    }
    fn material_flags(&self, _material_id: u8) -> Option<u32> {
        Some(2)
    }
}
fn put(bytes: &mut [u8], at: usize, value: u32) {
    bytes[at..at + 4].copy_from_slice(&value.to_le_bytes());
}
fn words(values: &[u32]) -> Vec<u8> {
    values.iter().flat_map(|v| v.to_le_bytes()).collect()
}
fn chunk(tag: &[u8; 4], body: &[u8]) -> Vec<u8> {
    let mut out: Vec<u8> = tag.iter().rev().copied().collect();
    out.extend_from_slice(&(body.len() as u32).to_le_bytes());
    out.extend_from_slice(body);
    out
}
fn root_bytes(ids: &[u32]) -> Vec<u8> {
    let mut header = [0; 64];
    put(&mut header, 4, ids.len() as u32);
    put(&mut header, 32, 0x1234_abcd);
    header[60] = 4;
    let mut bytes = chunk(b"MOHD", &header);
    bytes.extend(chunk(b"GFID", &words(ids)));
    bytes
}
fn surface(dims: [u32; 4]) -> Vec<u8> {
    let mut bytes = words(&dims);
    bytes.extend(words(&[0x8000_0000, 1, 0x3f80_0000]));
    bytes.extend_from_slice(&(-3i16).to_le_bytes());
    for index in 0..dims[0] * dims[1] {
        bytes.extend_from_slice(&[7, 0, 9, 0]);
        bytes.extend_from_slice(&(0x3f80_0000 + index).to_le_bytes());
    }
    bytes.extend(std::iter::repeat_n(0x8f, (dims[2] * dims[3]) as usize));
    bytes
}
fn group_bytes(id: u32, flags: u32, faces: bool, kind: u32, liquid: Option<&[u8]>) -> Vec<u8> {
    let mut body = vec![0; 68];
    put(&mut body, 8, flags);
    put(&mut body, 52, kind);
    put(&mut body, 56, id);
    body[64..66].copy_from_slice(&(-1i16).to_le_bytes());
    if faces {
        body.extend(chunk(b"MPY2", &[8, 0, 0, 0]));
        body.extend(chunk(b"MOVX", &words(&[2, 0, 1])));
        body.extend(chunk(
            b"MOVT",
            &words(&[0x8000_0000, 1, 0, 0x3f80_0000, 0, 0, 0, 0x3f80_0000, 0]),
        ));
        let mut batch = [0; 24];
        batch[16..18].copy_from_slice(&513u16.to_le_bytes());
        body.extend(chunk(b"MOBA", &batch));
    }
    if let Some(liquid) = liquid {
        body.extend(chunk(b"MLIQ", liquid));
    }
    chunk(b"MOGP", &body)
}
fn converted(
    root_data: &[u8],
    group_data: &[Vec<u8>],
    lookup: &Lookup,
    limits: ConversionLimits,
) -> Result<Option<ConvertedWmo>, Error> {
    let root = Root::parse(root_data, Limits::default())?;
    let groups: Vec<_> = group_data
        .iter()
        .map(|b| Group::parse(b, Limits::default()))
        .collect::<Result<_, _>>()?;
    let inputs: Vec<_> = root
        .detailed_group_ids()
        .zip(&groups)
        .map(|(id, group)| GroupInput {
            file_data_id: id,
            group,
        })
        .collect();
    convert(&root, &inputs, lookup, limits)
}
fn basic() -> Vec<u8> {
    converted(
        &root_bytes(&[77]),
        &[group_bytes(9, 0, true, 0, None)],
        &Lookup::empty(),
        ConversionLimits::default(),
    )
    .unwrap_or_else(|e| panic!("{e}"))
    .unwrap_or_else(|| panic!("missing collision"))
    .into_bytes()
}
fn parse(bytes: &[u8]) -> RawWmo<'_> {
    RawWmo::parse(bytes, ConversionLimits::default()).unwrap_or_else(|e| panic!("{e}"))
}
fn rejected(bytes: &[u8]) -> bool {
    RawWmo::parse(bytes, ConversionLimits::default()).is_err()
}

// Literal wire transcript independent from writer size/helper calculations.
fn golden_group() -> Vec<u8> {
    let mut bytes = words(&[0, 9, 0, 0, 0, 0, 0, 0, 0]);
    bytes.extend_from_slice(b"GRP ");
    bytes.extend(words(&[8, 1, 513]));
    bytes.extend_from_slice(b"INDX");
    bytes.extend(words(&[10, 3, 2, 0, 1]));
    bytes.extend_from_slice(b"VERT");
    bytes.extend(words(&[
        40,
        3,
        0x8000_0000,
        1,
        0,
        0x3f80_0000,
        0,
        0,
        0,
        0x3f80_0000,
        0,
    ]));
    bytes
}
fn golden() -> Vec<u8> {
    let mut bytes = b"VMAP04E\0".to_vec();
    bytes.extend(words(&[1, 1, 0x1234_abcd, 0]));
    bytes.extend(golden_group());
    bytes
}

#[test]
fn literal_complete_raw_golden_preserves_bits_winding_and_declared_anomaly() {
    let bytes = basic();
    assert_eq!(bytes, golden());
    assert_eq!(bytes.len(), 148);
    let raw = parse(&bytes);
    assert_eq!(
        (raw.triangle_count(), raw.group_count(), raw.root_wmo_id()),
        (1, 1, 0x1234_abcd)
    );
    assert_eq!(raw.bytes(), bytes);
    let group = raw.groups().next().unwrap();
    assert_eq!(group.group_wmo_id(), 9);
    assert_eq!(group.bounds_bits(), [0; 6]);
    assert_eq!(group.vertex_bits(0), Some([0x8000_0000, 1, 0]));
    assert_eq!(
        (group.index(0), group.index(1), group.index(2)),
        (Some(2), Some(0), Some(1))
    );
    assert_eq!(group.batch(0), Some(513));
    assert_eq!(group.liquid_flags(), 0);
    assert!(group.liquid().is_none());
}

#[test]
fn includes_empty_groups_when_any_group_has_collision_and_preserves_alias_order() {
    let bytes = converted(
        &root_bytes(&[7, 0, 7, 8]),
        &[
            group_bytes(50, 0, false, 0, None),
            group_bytes(51, 0, true, 0, None),
            group_bytes(52, 0, false, 0, None),
        ],
        &Lookup::empty(),
        ConversionLimits::default(),
    )
    .unwrap()
    .unwrap();
    assert_eq!((bytes.group_count(), bytes.triangle_count()), (3, 1));
    let raw = parse(bytes.bytes());
    let groups: Vec<_> = raw.groups().collect();
    assert_eq!(
        groups
            .iter()
            .map(RawGroup::group_wmo_id)
            .collect::<Vec<_>>(),
        [50, 51, 52]
    );
    assert_eq!(
        groups
            .iter()
            .map(RawGroup::triangle_count)
            .collect::<Vec<_>>(),
        [0, 1, 0]
    );
    assert_eq!(groups[0].vertex_count(), 0);
}

#[test]
fn no_collision_returns_none_for_empty_skipped_and_liquid_only_sources() {
    for data in [
        vec![],
        vec![group_bytes(1, 0, false, 0, None)],
        vec![group_bytes(1, 0x80, true, 21, None)],
        vec![group_bytes(1, 0, false, 21, Some(&surface([2, 2, 1, 1])))],
        // The consumer shape gate applies only to a surface actually emitted.
        vec![group_bytes(1, 0, false, 21, Some(&surface([0, 0, 0, 0])))],
    ] {
        let ids = vec![7; data.len()];
        assert!(
            converted(
                &root_bytes(&ids),
                &data,
                &Lookup::empty(),
                ConversionLimits {
                    max_raw_bytes: 0,
                    ..ConversionLimits::default()
                }
            )
            .unwrap()
            .is_none()
        );
    }
}

#[test]
fn split_ids_and_skips_use_resolved_contexts_without_reordering() {
    let a = group_bytes(51, 0, true, 0, None);
    let mut b = group_bytes(52, 0, true, 0, None);
    put(&mut b, 8 + 60, 0x80);
    b[8 + 64..8 + 66].copy_from_slice(&0i16.to_le_bytes());
    let c = group_bytes(53, 0x0400_0000, true, 21, None);
    let lookup = Lookup::empty();
    let out = converted(
        &root_bytes(&[7, 8, 9]),
        &[a, b, c],
        &lookup,
        ConversionLimits::default(),
    )
    .unwrap()
    .unwrap();
    let raw = parse(out.bytes());
    assert_eq!(
        raw.groups().map(|g| g.group_wmo_id()).collect::<Vec<_>>(),
        [51, 51]
    );
    // Header liquid checks for skipped groups still run, exactly once per group
    // projection; no second pass through lookup for geometry serialization.
    assert_eq!(lookup.queries.get(), 1);
}

#[test]
fn strict_inventory_rejects_missing_extra_or_wrong_group_ids() {
    let r = root_bytes(&[7]);
    let g = group_bytes(9, 0, true, 0, None);
    let root = Root::parse(&r, Limits::default()).unwrap();
    let group = Group::parse(&g, Limits::default()).unwrap();
    for inputs in [
        vec![],
        vec![GroupInput {
            file_data_id: 8,
            group: &group,
        }],
        vec![
            GroupInput {
                file_data_id: 7,
                group: &group
            };
            2
        ],
    ] {
        assert_eq!(
            convert(
                &root,
                &inputs,
                &Lookup::empty(),
                ConversionLimits::default()
            )
            .err(),
            Some(Error::Inventory)
        );
    }
}

#[test]
fn liquid_flags_one_two_three_have_exact_independent_wire_transcripts() {
    let surf = surface([2, 2, 1, 1]);
    for (kind, view, flags, type_id) in [
        (0, Some(surf.as_slice()), 1, 0),
        (21, None, 2, 21),
        (21, Some(surf.as_slice()), 3, 21),
    ] {
        let out = converted(
            &root_bytes(&[7]),
            &[group_bytes(9, 0, true, kind, view)],
            &Lookup::empty(),
            ConversionLimits::default(),
        )
        .unwrap()
        .unwrap();
        let mut expected = golden();
        put(&mut expected, 56, flags);
        expected.extend_from_slice(b"LIQU");
        if flags & 1 == 0 {
            expected.extend(words(&[4, type_id]));
        } else {
            expected.extend(words(&[51, type_id]));
            expected.extend_from_slice(&surf[..30]);
            expected.extend(words(&[0x3f80_0000, 0x3f80_0001, 0x3f80_0002, 0x3f80_0003]));
            expected.push(0x8f);
        }
        assert_eq!(out.bytes(), expected);
        let raw = parse(out.bytes());
        let group = raw.groups().next().unwrap();
        let liquid = group.liquid().unwrap();
        assert_eq!((group.liquid_flags(), liquid.type_id()), (flags, type_id));
        assert_eq!(liquid.dimensions(), view.map(|_| [2, 2, 1, 1]));
        if view.is_some() {
            assert_eq!(liquid.material(), Some(-3));
            assert_eq!(liquid.position_bits(), Some([0x8000_0000, 1, 0x3f80_0000]));
            assert_eq!(liquid.height_bits(3), Some(0x3f80_0003));
            assert_eq!(liquid.tiles(), [0x8f]);
        } else {
            assert_eq!(liquid.height_count(), 0);
            assert!(liquid.tiles().is_empty());
        }
    }
}

#[test]
fn liquid_lookup_decisions_are_not_repeated_in_serialization_pass() {
    let lookup = Lookup::empty();
    let out = converted(
        &root_bytes(&[7]),
        &[group_bytes(9, 0, true, 21, Some(&surface([2, 2, 1, 1])))],
        &lookup,
        ConversionLimits::default(),
    )
    .unwrap()
    .unwrap();
    assert_eq!(lookup.queries.get(), 2); // header + MLIQ, once source projection
    let raw = parse(out.bytes());
    assert_eq!(raw.groups().next().unwrap().liquid_flags(), 3);
    assert_eq!(lookup.queries.get(), 2); // raw reader never consults lookups
}

#[test]
fn ignored_liquid_has_no_chunk_and_does_not_require_raw_consumer_shape() {
    let lookup = Lookup {
        queries: Cell::new(0),
        ignored: Some(21),
    };
    let out = converted(
        &root_bytes(&[7]),
        &[group_bytes(9, 0, true, 21, Some(&surface([0, 0, 0, 0])))],
        &lookup,
        ConversionLimits::default(),
    )
    .unwrap()
    .unwrap();
    assert_eq!(out.bytes(), golden());
}

#[test]
fn emitted_liquid_requires_nonzero_tiles_and_exact_assembler_height_capacity() {
    for dims in [
        [0, 0, 0, 0],
        [1, 1, 0, 0],
        [1, 1, 0, 1],
        [1, 1, 1, 1],
        [3, 2, 1, 1],
    ] {
        assert_eq!(
            converted(
                &root_bytes(&[7]),
                &[group_bytes(9, 0, true, 21, Some(&surface(dims)))],
                &Lookup::empty(),
                ConversionLimits::default()
            )
            .err(),
            Some(Error::Shape)
        );
    }
    // Product, not per-axis equality, controls the native height storage read.
    let out = converted(
        &root_bytes(&[7]),
        &[group_bytes(9, 0, true, 21, Some(&surface([1, 4, 1, 1])))],
        &Lookup::empty(),
        ConversionLimits::default(),
    )
    .unwrap()
    .unwrap();
    assert_eq!(
        parse(out.bytes())
            .groups()
            .next()
            .unwrap()
            .liquid()
            .unwrap()
            .dimensions(),
        Some([1, 4, 1, 1])
    );
}

#[test]
fn raw_reader_rejects_every_truncated_prefix_and_trailing_bytes() {
    let bytes = basic();
    for end in 0..bytes.len() {
        assert!(rejected(&bytes[..end]), "prefix {end}");
    }
    let mut extra = bytes;
    extra.push(0);
    assert!(rejected(&extra));
}

#[test]
fn raw_header_totals_flags_tags_and_sizes_are_checked() {
    let original = basic();
    for (at, value) in [
        (8, 0),
        (8, 2),
        (12, 0),
        (12, 2),
        (20, 1),
        (56, 4),
        (64, 9),
        (68, 2),
        (72, 65536),
        (80, 16),
        (84, 2),
        (104, 41),
        (108, 4),
    ] {
        let mut bytes = original.clone();
        put(&mut bytes, at, value);
        assert!(rejected(&bytes), "offset {at}");
    }
    for at in [0, 60, 76, 100] {
        let mut bytes = original.clone();
        bytes[at] ^= 1;
        assert!(rejected(&bytes));
    }
}

#[test]
fn raw_reader_checks_indices_nonfinite_and_inverted_bounds() {
    let original = basic();
    for (at, value) in [
        (88, 3),
        (88, u32::MAX),
        (112, 0x7f80_0000),
        (116, 0x7fc0_1234),
        (32, 0x3f80_0000),
        (44, 0xff80_0000),
    ] {
        let mut bytes = original.clone();
        put(&mut bytes, at, value);
        assert!(rejected(&bytes));
    }
}

#[test]
fn raw_reader_admits_wide_index_domain_and_rejects_its_out_of_bounds() {
    let mut bytes = b"VMAP04E\0".to_vec();
    bytes.extend(words(&[1, 1, 9, 0]));
    bytes.extend(words(&[0, 9, 0, 0, 0, 0, 0, 0, 0]));
    bytes.extend_from_slice(b"GRP ");
    bytes.extend(words(&[4, 0]));
    bytes.extend_from_slice(b"INDX");
    bytes.extend(words(&[10, 3, 65536, 0, 1]));
    bytes.extend_from_slice(b"VERT");
    bytes.extend(words(&[4 + 65537 * 12, 65537]));
    bytes.resize(bytes.len() + 65537 * 12, 0);
    let raw = parse(&bytes);
    assert_eq!(raw.groups().next().unwrap().index(0), Some(65536));
    put(&mut bytes, 84, 65537);
    assert!(rejected(&bytes));
}

#[test]
fn raw_reader_liquid_extent_dimensions_and_finite_values_are_strict() {
    let out = converted(
        &root_bytes(&[7]),
        &[group_bytes(9, 0, true, 21, Some(&surface([2, 2, 1, 1])))],
        &Lookup::empty(),
        ConversionLimits::default(),
    )
    .unwrap()
    .unwrap();
    let original = out.bytes();
    // LIQU begins at 148; type at156, packed header at160, heights at190.
    for (at, value) in [
        (152, 50),
        (160, u32::MAX),
        (160, 1),
        (168, 0),
        (176, 0x7fc0_0001),
        (190, 0x7f80_0000),
    ] {
        let mut bytes = original.to_vec();
        put(&mut bytes, at, value);
        assert!(rejected(&bytes), "offset {at}");
    }
    for end in 148..original.len() {
        assert!(rejected(&original[..end]));
    }
    let mut noflag = original.to_vec();
    put(&mut noflag, 56, 0);
    assert!(rejected(&noflag));
    let mut headeronly = original.to_vec();
    put(&mut headeronly, 56, 2);
    assert!(rejected(&headeronly));
}

#[test]
fn getters_reject_extreme_indices_without_arithmetic_wrap() {
    let bytes = basic();
    let raw = parse(&bytes);
    let group = raw.groups().next().unwrap();
    for i in [3, usize::MAX] {
        assert!(group.index(i).is_none());
        assert!(group.vertex_bits(i).is_none());
    }
    assert!(group.batch(usize::MAX).is_none());
    let out = converted(
        &root_bytes(&[7]),
        &[group_bytes(9, 0, true, 21, Some(&surface([2, 2, 1, 1])))],
        &Lookup::empty(),
        ConversionLimits::default(),
    )
    .unwrap()
    .unwrap();
    let raw = parse(out.bytes());
    let group = raw.groups().next().unwrap();
    let liquid = group.liquid().unwrap();
    assert!(liquid.height_bits(4).is_none());
    assert!(liquid.height_bits(usize::MAX).is_none());
}

#[test]
fn output_and_source_limits_only_reduce_and_enforce_exact_raw_size() {
    let r = root_bytes(&[7]);
    let g = [group_bytes(9, 0, true, 0, None)];
    let mut limits = ConversionLimits {
        max_raw_bytes: 148,
        ..ConversionLimits::default()
    };
    assert_eq!(
        converted(&r, &g, &Lookup::empty(), limits)
            .unwrap()
            .unwrap()
            .bytes()
            .len(),
        148
    );
    limits.max_raw_bytes = 147;
    assert_eq!(
        converted(&r, &g, &Lookup::empty(), limits).err(),
        Some(Error::Limit)
    );
    assert_eq!(RawWmo::parse(&golden(), limits).err(), Some(Error::Limit));
    for raised in [
        ConversionLimits {
            max_raw_bytes: MAX_RAW_BYTES + 1,
            ..ConversionLimits::default()
        },
        ConversionLimits {
            max_memory_bytes: MAX_MEMORY_BYTES + 1,
            ..ConversionLimits::default()
        },
        ConversionLimits {
            source: Limits {
                max_vertices: super::super::MAX_VERTICES + 1,
                ..Limits::default()
            },
            ..ConversionLimits::default()
        },
    ] {
        assert_eq!(
            converted(&r, &g, &Lookup::empty(), raised).err(),
            Some(Error::InvalidLimits)
        );
        assert_eq!(
            RawWmo::parse(&golden(), raised).err(),
            Some(Error::InvalidLimits)
        );
    }
    for source in [
        Limits {
            max_input_bytes: 1,
            ..Limits::default()
        },
        Limits {
            max_vertices: 2,
            ..Limits::default()
        },
        Limits {
            max_indices: 2,
            ..Limits::default()
        },
        Limits {
            max_metadata_records: 0,
            ..Limits::default()
        },
        Limits {
            max_chunks: 1,
            ..Limits::default()
        },
        Limits {
            max_groups: 0,
            ..Limits::default()
        },
    ] {
        assert_eq!(
            converted(
                &r,
                &g,
                &Lookup::empty(),
                ConversionLimits {
                    source,
                    ..ConversionLimits::default()
                }
            )
            .err(),
            Some(Error::Limit)
        );
    }
}

#[test]
fn combined_memory_budget_charges_contexts_plan_output_and_projection_scratch() {
    let r = root_bytes(&[7]);
    let g = [group_bytes(9, 0, true, 0, None)];
    // Context+plan capacities, final148bytes, and geometry peak:
    // 3 renumber words + 3 vertices + 3 indices + 1 batch value = 64.
    let overhead = size_of::<super::super::GroupContext<'_, '_>>() + size_of::<Plan<'_>>();
    let exact = overhead + 148 + 64;
    for cap in [0, overhead - 1, overhead + 147, exact - 1] {
        assert_eq!(
            converted(
                &r,
                &g,
                &Lookup::empty(),
                ConversionLimits {
                    max_memory_bytes: cap,
                    ..ConversionLimits::default()
                }
            )
            .err(),
            Some(Error::Limit)
        );
    }
    assert!(
        converted(
            &r,
            &g,
            &Lookup::empty(),
            ConversionLimits {
                max_memory_bytes: exact,
                ..ConversionLimits::default()
            }
        )
        .unwrap()
        .is_some()
    );
    // Reader is borrowed: it needs no heap allowance.
    assert!(
        RawWmo::parse(
            &golden(),
            ConversionLimits {
                max_memory_bytes: 0,
                ..ConversionLimits::default()
            }
        )
        .is_ok()
    );
}

#[test]
fn geometry_subtotal_rejects_before_later_liquid_lookups_once_collision_exists() {
    for empty_first in [false, true] {
        let lookup = Lookup::empty();
        let groups = [
            group_bytes(1, 0, !empty_first, 21, None),
            group_bytes(2, 0, true, 21, None),
            group_bytes(3, 0, true, 21, None),
        ];
        let limits = ConversionLimits {
            max_raw_bytes: 147,
            ..ConversionLimits::default()
        };
        assert_eq!(
            converted(&root_bytes(&[7, 8, 9]), &groups, &lookup, limits).err(),
            Some(Error::Limit)
        );
        assert_eq!(lookup.queries.get(), if empty_first { 2 } else { 1 });
    }

    // Empty emitted group headers exceed this cap, but no raw file is needed.
    let lookup = Lookup::empty();
    let groups = [
        group_bytes(1, 0, false, 21, None),
        group_bytes(2, 0, false, 21, None),
        group_bytes(3, 0, false, 21, None),
    ];
    assert!(
        converted(
            &root_bytes(&[7, 8, 9]),
            &groups,
            &lookup,
            ConversionLimits {
                max_raw_bytes: 0,
                ..ConversionLimits::default()
            }
        )
        .unwrap()
        .is_none()
    );
    assert_eq!(lookup.queries.get(), 3);
}

#[test]
fn complete_source_validation_precedes_no_collision_and_lookup() {
    let r = root_bytes(&[7]);
    let mut g = group_bytes(9, 0, false, 21, None);
    g.push(0);
    let lookup = Lookup::empty();
    assert!(converted(&r, &[g], &lookup, ConversionLimits::default()).is_err());
    assert_eq!(lookup.queries.get(), 0);
}
