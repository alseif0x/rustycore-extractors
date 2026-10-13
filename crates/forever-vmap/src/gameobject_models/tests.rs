// SPDX-License-Identifier: GPL-3.0-or-later
// Authored 2026-10-11: independent synthetic wire fixtures; no client assets.

use super::*;

fn word_out(out: &mut Vec<u8>, value: u32) {
    out.extend_from_slice(&value.to_le_bytes());
}
fn set_word(out: &mut [u8], at: usize, value: u32) {
    out[at..at + 4].copy_from_slice(&value.to_le_bytes());
}
fn bits(values: [f32; 6]) -> [u32; 6] {
    values.map(f32::to_bits)
}
fn triangle() -> Vec<[u32; 3]> {
    [[-2.0, 3.0, 4.0], [5.0, -6.0, 7.0], [1.0, 2.0, -8.0]]
        .map(|v| v.map(f32::to_bits))
        .to_vec()
}
fn group(vertices: &[[u32; 3]]) -> Vec<u8> {
    let mut out = Vec::new();
    word_out(&mut out, 0);
    word_out(&mut out, 0);
    // Deliberately unrelated declared bounds. Bounds do not contain the mesh.
    for b in bits([100.0, 100.0, 100.0, 101.0, 101.0, 101.0]) {
        word_out(&mut out, b);
    }
    word_out(&mut out, 0);
    out.extend_from_slice(b"GRP ");
    word_out(&mut out, 8);
    word_out(&mut out, 1);
    let indices = if vertices.is_empty() { 0 } else { 3 };
    word_out(&mut out, indices);
    out.extend_from_slice(b"INDX");
    word_out(&mut out, 4 + 2 * indices);
    word_out(&mut out, indices);
    for i in 0..indices {
        word_out(&mut out, i);
    }
    out.extend_from_slice(b"VERT");
    word_out(&mut out, 4 + 12 * vertices.len() as u32);
    word_out(&mut out, vertices.len() as u32);
    for v in vertices {
        for &b in v {
            word_out(&mut out, b);
        }
    }
    out
}
fn m2(vertices: &[[u32; 3]]) -> Vec<u8> {
    let mut out = b"VMAP04E\0".to_vec();
    for w in [vertices.len() as u32, 1, 0, 1] {
        word_out(&mut out, w);
    }
    out.extend(group(vertices));
    out
}
fn wmo(groups: &[Vec<u8>], triangles: u32) -> Vec<u8> {
    let mut out = b"VMAP04E\0".to_vec();
    for w in [triangles, groups.len() as u32, 123, 0] {
        word_out(&mut out, w);
    }
    for g in groups {
        out.extend_from_slice(g);
    }
    out
}
fn resolved<'a>(id: u32, name: &'a [u8], raw: &'a [u8]) -> ResolvedRow<'a> {
    ResolvedRow {
        display_id: id,
        name,
        model: Some(RawModel::M2(raw)),
    }
}
fn list(final_list: bool, rows: &[(u32, &[u8], [u32; 6])]) -> Vec<u8> {
    let mut out = if final_list {
        b"VMAP_4.E".to_vec()
    } else {
        b"VMAP04E\0".to_vec()
    };
    for (id, name, bounds) in rows {
        word_out(&mut out, *id);
        word_out(&mut out, name.len() as u32);
        out.extend_from_slice(name);
        if final_list {
            for &b in bounds {
                word_out(&mut out, b);
            }
        }
    }
    out
}
fn result_bounds(lists: &Lists) -> Vec<[u32; 6]> {
    Dtree::parse(lists.dtree_bytes(), Limits::default())
        .unwrap()
        .rows()
        .map(|r| r.bounds_bits())
        .collect()
}

#[test]
fn m2_literal_wire_goldens_and_stats() {
    let raw = m2(&triangle());
    assert_eq!(raw.len(), 148);
    let output = build(&[resolved(0x12345678, b"a", &raw)], Limits::default()).unwrap();
    let mut temp = b"VMAP04E\0\x78\x56\x34\x12\x01\0\0\0a".to_vec();
    assert_eq!(output.temp_bytes(), temp);
    temp[..8].copy_from_slice(b"VMAP_4.E");
    temp.extend_from_slice(&[
        0, 0, 0, 0xc0, 0, 0, 0xc0, 0xc0, 0, 0, 0, 0xc1, 0, 0, 0xa0, 0x40, 0, 0, 0x40, 0x40, 0, 0,
        0xe0, 0x40,
    ]);
    assert_eq!(output.dtree_bytes(), temp);
    let s = output.stats();
    assert_eq!(
        (
            s.rows,
            s.m2_rows,
            s.wmo_rows,
            s.vertex_visits,
            s.raw_visit_bytes
        ),
        (1, 1, 0, 3, 148)
    );
    assert_eq!((s.temp_bytes, s.dtree_bytes), (17, 41));
    assert_eq!(s.peak_bytes, s.resident_bytes + 24);
}

#[test]
fn unreferenced_vertices_and_declared_bounds_do_not_limit_extrema() {
    let mut vertices = triangle();
    vertices.push([-90.0f32, 80.0, -70.0].map(f32::to_bits));
    let raw = m2(&vertices);
    let output = build(&[resolved(1, b"model", &raw)], Limits::default()).unwrap();
    assert_eq!(
        result_bounds(&output),
        [bits([-90.0, -6.0, -70.0, 5.0, 80.0, 7.0])]
    );
    assert_eq!(output.stats().vertex_visits, 4);
}

#[test]
fn wmo_every_group_and_empty_group_are_walked_in_physical_order() {
    let mut extra = triangle();
    extra.push([40.0f32, -50.0, 60.0].map(f32::to_bits));
    let raw = wmo(
        &[group(&[]), group(&triangle()), group(&extra), group(&[])],
        2,
    );
    let output = build(
        &[ResolvedRow {
            display_id: u32::MAX,
            name: b"w",
            model: Some(RawModel::Wmo(&raw)),
        }],
        Limits::default(),
    )
    .unwrap();
    assert_eq!(
        result_bounds(&output),
        [bits([-2.0, -50.0, -8.0, 40.0, 3.0, 60.0])]
    );
    assert_eq!(
        (output.stats().wmo_rows, output.stats().vertex_visits),
        (1, 7)
    );
}

#[test]
fn last_equal_vertex_wins_all_signed_zero_extrema() {
    for final_bits in [0, 0x80000000] {
        let previous = final_bits ^ 0x80000000;
        let raw = wmo(
            &[group(&[[previous; 3]; 3]), group(&[[final_bits; 3]; 3])],
            2,
        );
        let output = build(
            &[ResolvedRow {
                display_id: 0,
                name: b"zero",
                model: Some(RawModel::Wmo(&raw)),
            }],
            Limits::default(),
        )
        .unwrap();
        assert_eq!(result_bounds(&output), [[final_bits; 6]]);
    }
}

#[test]
fn duplicate_ids_and_names_preserve_every_ordered_row() {
    // Deliberately independent row models: exercises this codec's row contract,
    // not native inventory parity for one filename referring to one file.
    let a = m2(&triangle());
    let b = m2(&[[0; 3]; 3]);
    let rows = [
        resolved(9, b"same", &a),
        resolved(0, b"second", &b),
        resolved(9, b"same", &b),
        resolved(u32::MAX, b"same", &a),
    ];
    let output = build(&rows, Limits::default()).unwrap();
    let parsed = Dtree::parse(output.dtree_bytes(), Limits::default()).unwrap();
    assert_eq!(
        parsed.rows().map(|r| r.display_id()).collect::<Vec<_>>(),
        [9, 0, 9, u32::MAX]
    );
    assert_eq!(
        parsed.rows().map(|r| r.name()).collect::<Vec<_>>(),
        [&b"same"[..], b"second", b"same", b"same"]
    );
    assert_eq!(output.stats().vertex_visits, 12);
    assert_eq!(
        output.stats().raw_visit_bytes,
        (a.len() * 2 + b.len() * 2) as u64
    );
}

#[test]
fn empty_lists_have_only_magic_and_exact_memory_minimum() {
    let limits = Limits {
        max_memory_bytes: 16,
        max_rows: 0,
        max_name_bytes: 0,
        max_vertex_visits: 0,
        max_raw_visit_bytes: 0,
        max_raw_bytes: 0,
        ..Limits::default()
    };
    let output = build(&[], limits).unwrap();
    assert_eq!(output.temp_bytes(), b"VMAP04E\0");
    assert_eq!(output.dtree_bytes(), b"VMAP_4.E");
    assert_eq!(output.stats().resident_bytes, 16);
    assert_eq!(output.stats().peak_bytes, 16);
    assert_eq!(
        build(
            &[],
            Limits {
                max_memory_bytes: 15,
                ..limits
            }
        )
        .err(),
        Some(Error::Limit)
    );
    let read_limits = Limits {
        max_memory_bytes: 0,
        ..limits
    };
    assert_eq!(
        TempList::parse(output.temp_bytes(), read_limits)
            .unwrap()
            .row_count(),
        0
    );
    assert_eq!(
        Dtree::parse(output.dtree_bytes(), read_limits)
            .unwrap()
            .row_count(),
        0
    );
    assert_eq!(
        build(
            &[],
            Limits {
                max_list_bytes: 7,
                ..limits
            }
        )
        .err(),
        Some(Error::Limit)
    );
}

#[test]
fn borrowed_rows_have_exact_fused_iteration_and_original_pointers() {
    let bytes = list(false, &[(1, b"a", [0; 6]), (2, b"abc", [0; 6])]);
    let parsed = TempList::parse(&bytes, Limits::default()).unwrap();
    assert_eq!(parsed.bytes().as_ptr(), bytes.as_ptr());
    let mut rows = parsed.rows();
    assert_eq!(rows.len(), 2);
    assert_eq!(rows.next().unwrap().name().as_ptr(), bytes[16..].as_ptr());
    assert_eq!(rows.len(), 1);
    rows.next().unwrap();
    assert_eq!(rows.len(), 0);
    assert!(rows.next().is_none());
    assert!(rows.next().is_none());
    let final_bytes = list(true, &[(1, b"a", [0; 6])]);
    let final_list = Dtree::parse(&final_bytes, Limits::default()).unwrap();
    assert_eq!(final_list.bytes().as_ptr(), final_bytes.as_ptr());
    let mut rows = final_list.rows();
    assert_eq!(rows.len(), 1);
    rows.next().unwrap();
    assert_eq!(rows.size_hint(), (0, Some(0)));
    assert!(rows.next().is_none());
    assert!(rows.next().is_none());
}

#[test]
fn ascii_name_limits_and_path_spelling_are_opaque() {
    let raw = m2(&triangle());
    let long = vec![b'a'; 499];
    for name in [&b"x"[..], &long, b"../a\\B.m2", b"a/b", b"\x01\x7f"] {
        let output = build(&[resolved(1, name, &raw)], Limits::default()).unwrap();
        assert_eq!(
            TempList::parse(output.temp_bytes(), Limits::default())
                .unwrap()
                .rows()
                .next()
                .unwrap()
                .name(),
            name
        );
    }
    for bad in [&b""[..], b"a\0b", b"\x80"] {
        assert_eq!(
            build(&[resolved(1, bad, &raw)], Limits::default()).err(),
            Some(Error::Name)
        );
        for final_list in [false, true] {
            let bytes = list(final_list, &[(1, bad, [0; 6])]);
            assert_eq!(
                validate_list(&bytes, final_list, Limits::default()).err(),
                Some(Error::Name)
            );
        }
    }
    assert_eq!(
        build(&[resolved(1, &vec![b'a'; 500], &raw)], Limits::default()).err(),
        Some(Error::Limit)
    );
}

#[test]
fn all_single_row_truncations_except_empty_header_are_rejected() {
    for final_list in [false, true] {
        let bytes = list(final_list, &[(1, b"abcd", [0; 6])]);
        for cut in 0..bytes.len() {
            if cut == 8 {
                continue;
            }
            assert!(
                validate_list(&bytes[..cut], final_list, Limits::default()).is_err(),
                "cut {cut}"
            );
        }
        let mut tail = bytes.clone();
        tail.push(0);
        assert_eq!(
            validate_list(&tail, final_list, Limits::default()).err(),
            Some(Error::Framing)
        );
    }
}

#[test]
fn magic_and_declared_name_extent_are_strict() {
    for final_list in [false, true] {
        let mut bytes = list(final_list, &[(1, b"x", [0; 6])]);
        bytes[0] = 0;
        assert_eq!(
            validate_list(&bytes, final_list, Limits::default()).err(),
            Some(Error::Framing)
        );
        let mut bytes = list(final_list, &[(1, b"x", [0; 6])]);
        set_word(&mut bytes, 12, 499);
        assert_eq!(
            validate_list(&bytes, final_list, Limits::default()).err(),
            Some(Error::Framing)
        );
        set_word(&mut bytes, 12, u32::MAX);
        assert_eq!(
            validate_list(&bytes, final_list, Limits::default()).err(),
            Some(Error::Limit)
        );
    }
}

#[test]
fn dtree_bounds_require_finite_ordered_values_without_zero_canonicalization() {
    for bad in [
        f32::INFINITY.to_bits(),
        f32::NEG_INFINITY.to_bits(),
        0x7fc00001,
    ] {
        for index in 0..6 {
            let mut bounds = [0; 6];
            bounds[index] = bad;
            assert_eq!(
                Dtree::parse(&list(true, &[(1, b"a", bounds)]), Limits::default()).err(),
                Some(Error::NonFinite)
            );
        }
    }
    assert_eq!(
        Dtree::parse(
            &list(true, &[(1, b"a", bits([1.0, 0.0, 0.0, 0.0, 0.0, 0.0]))]),
            Limits::default()
        )
        .err(),
        Some(Error::InvertedBounds)
    );
    let bytes = list(
        true,
        &[(1, b"a", [0, 0x80000000, 0, 0x80000000, 0, 0x80000000])],
    );
    assert_eq!(
        Dtree::parse(&bytes, Limits::default())
            .unwrap()
            .rows()
            .next()
            .unwrap()
            .bounds_bits(),
        [0, 0x80000000, 0, 0x80000000, 0, 0x80000000]
    );
}

#[test]
fn missing_model_and_late_malformed_raw_fail_whole_build() {
    let raw = m2(&triangle());
    let absent = ResolvedRow {
        display_id: 2,
        name: b"missing",
        model: None,
    };
    assert_eq!(
        build(&[absent], Limits::default()).err(),
        Some(Error::MissingModel)
    );
    assert_eq!(
        build(&[resolved(1, b"good", &raw), absent], Limits::default()).err(),
        Some(Error::MissingModel)
    );
    assert!(matches!(
        build(
            &[resolved(1, b"good", &raw), resolved(2, b"bad", b"broken")],
            Limits::default()
        ),
        Err(Error::RawM2(_))
    ));
}

#[test]
fn dependency_preflight_precedes_any_raw_validation_or_reservation() {
    let malformed = resolved(1, b"bad", b"broken");
    let absent = ResolvedRow {
        display_id: 2,
        name: b"missing",
        model: None,
    };
    assert_eq!(
        build(
            &[malformed, absent],
            Limits {
                max_memory_bytes: 0,
                ..Limits::default()
            }
        )
        .err(),
        Some(Error::MissingModel)
    );
    let later = resolved(3, b"later", &[0; 8]);
    assert_eq!(
        build(
            &[malformed, later],
            Limits {
                max_raw_visit_bytes: 13,
                ..Limits::default()
            }
        )
        .err(),
        Some(Error::Limit)
    );
    assert_eq!(
        build(
            &[malformed, later],
            Limits {
                max_raw_bytes: 7,
                ..Limits::default()
            }
        )
        .err(),
        Some(Error::Limit)
    );
    let bad_name = resolved(2, b"", b"");
    assert_eq!(
        build(&[malformed, bad_name], Limits::default()).err(),
        Some(Error::Name)
    );
}

#[test]
fn explicit_model_class_routing_is_not_a_fallback() {
    let raw_m2 = m2(&triangle());
    let raw_wmo = wmo(&[group(&triangle())], 1);
    assert!(matches!(
        build(
            &[ResolvedRow {
                display_id: 1,
                name: b"a",
                model: Some(RawModel::Wmo(&raw_m2))
            }],
            Limits::default()
        ),
        Err(Error::RawWmo(_))
    ));
    assert!(matches!(
        build(&[resolved(1, b"a", &raw_wmo)], Limits::default()),
        Err(Error::RawM2(_))
    ));
}

#[test]
fn empty_geometry_is_not_a_silently_omitted_success() {
    let empty_m2 = m2(&[]);
    let empty_wmo = wmo(&[group(&[])], 0);
    assert!(build(&[resolved(1, b"a", &empty_m2)], Limits::default()).is_err());
    assert!(
        build(
            &[ResolvedRow {
                display_id: 1,
                name: b"a",
                model: Some(RawModel::Wmo(&empty_wmo))
            }],
            Limits::default()
        )
        .is_err()
    );
}

#[test]
fn raw_reader_still_validates_unused_geometry_and_bounds() {
    let mut vertices = triangle();
    vertices.push([f32::NAN.to_bits(), 0, 0]);
    assert!(matches!(
        build(&[resolved(1, b"a", &m2(&vertices))], Limits::default()),
        Err(Error::RawM2(crate::Error::NonFinite))
    ));
    let mut raw = m2(&triangle());
    set_word(&mut raw, 88, 99);
    assert!(matches!(
        build(&[resolved(1, b"a", &raw)], Limits::default()),
        Err(Error::RawM2(crate::Error::Index))
    ));
    let mut raw = m2(&triangle());
    set_word(&mut raw, 32, f32::INFINITY.to_bits());
    assert!(matches!(
        build(&[resolved(1, b"a", &raw)], Limits::default()),
        Err(Error::RawM2(crate::Error::NonFinite))
    ));
    let mut raw = m2(&triangle());
    raw.push(0);
    assert!(matches!(
        build(&[resolved(1, b"a", &raw)], Limits::default()),
        Err(Error::RawM2(_))
    ));
}

#[test]
fn liquid_does_not_contribute_but_is_fully_validated() {
    let mut g = group(&triangle());
    set_word(&mut g, 32, 2);
    g.extend_from_slice(b"LIQU");
    word_out(&mut g, 4);
    word_out(&mut g, u32::MAX);
    let raw = wmo(&[g.clone()], 1);
    let row = ResolvedRow {
        display_id: 1,
        name: b"liquid",
        model: Some(RawModel::Wmo(&raw)),
    };
    assert_eq!(
        result_bounds(&build(&[row], Limits::default()).unwrap()),
        [bits([-2.0, -6.0, -8.0, 5.0, 3.0, 7.0])]
    );
    g.pop();
    let bad = wmo(&[g], 1);
    assert!(matches!(
        build(
            &[ResolvedRow {
                model: Some(RawModel::Wmo(&bad)),
                ..row
            }],
            Limits::default()
        ),
        Err(Error::RawWmo(_))
    ));
}

#[test]
fn row_name_raw_and_each_list_byte_caps_are_downward_exact() {
    let raw = m2(&triangle());
    let row = resolved(1, b"abc", &raw);
    let exact = Limits {
        max_rows: 1,
        max_name_bytes: 3,
        max_raw_bytes: raw.len(),
        max_list_bytes: 43,
        ..Limits::default()
    };
    build(&[row], exact).unwrap();
    for limits in [
        Limits {
            max_rows: 0,
            ..exact
        },
        Limits {
            max_name_bytes: 2,
            ..exact
        },
        Limits {
            max_raw_bytes: raw.len() - 1,
            ..exact
        },
        Limits {
            max_list_bytes: 42,
            ..exact
        },
    ] {
        assert_eq!(build(&[row], limits).err(), Some(Error::Limit));
    }
    let final_bytes = list(true, &[(1, b"abc", [0; 6])]);
    assert_eq!(
        Dtree::parse(
            &final_bytes,
            Limits {
                max_list_bytes: 42,
                ..exact
            }
        )
        .err(),
        Some(Error::Limit)
    );
    assert_eq!(
        Dtree::parse(
            &final_bytes,
            Limits {
                max_rows: 0,
                ..exact
            }
        )
        .err(),
        Some(Error::Limit)
    );
}

#[test]
fn aliased_raw_bytes_and_vertices_are_charged_again_per_row() {
    let raw = m2(&triangle());
    let row = resolved(1, b"a", &raw);
    let exact = Limits {
        max_vertex_visits: 6,
        max_raw_visit_bytes: (raw.len() * 2) as u64,
        ..Limits::default()
    };
    let output = build(&[row, row], exact).unwrap();
    assert_eq!(output.stats().raw_visit_bytes, 296);
    assert_eq!(
        build(
            &[row, row],
            Limits {
                max_vertex_visits: 5,
                ..exact
            }
        )
        .err(),
        Some(Error::Limit)
    );
    assert_eq!(
        build(
            &[row, row],
            Limits {
                max_raw_visit_bytes: 295,
                ..exact
            }
        )
        .err(),
        Some(Error::Limit)
    );
}

#[test]
fn wmo_vertex_budget_is_cumulative_across_groups() {
    let raw = wmo(&[group(&triangle()), group(&triangle())], 2);
    let row = ResolvedRow {
        display_id: 1,
        name: b"w",
        model: Some(RawModel::Wmo(&raw)),
    };
    build(
        &[row],
        Limits {
            max_vertex_visits: 6,
            ..Limits::default()
        },
    )
    .unwrap();
    assert_eq!(
        build(
            &[row],
            Limits {
                max_vertex_visits: 5,
                ..Limits::default()
            }
        )
        .err(),
        Some(Error::Limit)
    );
}

#[test]
fn heap_includes_both_output_capacities_and_all_temporary_bounds() {
    let raw = m2(&triangle());
    let rows = [resolved(1, b"a", &raw), resolved(2, b"longer", &raw)];
    let output = build(&rows, Limits::default()).unwrap();
    let stats = output.stats();
    assert_eq!(stats.peak_bytes, stats.resident_bytes + 48);
    let exact = Limits {
        max_memory_bytes: stats.peak_bytes,
        ..Limits::default()
    };
    let again = build(&rows, exact).unwrap();
    assert_eq!(again.temp_bytes(), output.temp_bytes());
    assert_eq!(
        build(
            &rows,
            Limits {
                max_memory_bytes: stats.peak_bytes - 1,
                ..exact
            }
        )
        .err(),
        Some(Error::Limit)
    );
    let (temp, dtree) = again.into_bytes();
    assert_eq!(temp, output.temp_bytes());
    assert_eq!(dtree, output.dtree_bytes());
}

#[test]
fn every_raised_ceiling_is_rejected_even_for_empty_input() {
    for limits in [
        Limits {
            max_list_bytes: MAX_LIST_BYTES + 1,
            ..Limits::default()
        },
        Limits {
            max_raw_bytes: MAX_RAW_BYTES + 1,
            ..Limits::default()
        },
        Limits {
            max_raw_visit_bytes: MAX_RAW_VISIT_BYTES + 1,
            ..Limits::default()
        },
        Limits {
            max_rows: MAX_ROWS + 1,
            ..Limits::default()
        },
        Limits {
            max_name_bytes: MAX_NAME_BYTES + 1,
            ..Limits::default()
        },
        Limits {
            max_vertex_visits: MAX_VERTEX_VISITS + 1,
            ..Limits::default()
        },
        Limits {
            max_memory_bytes: MAX_MEMORY_BYTES + 1,
            ..Limits::default()
        },
    ] {
        assert_eq!(build(&[], limits).err(), Some(Error::InvalidLimits));
        assert_eq!(
            TempList::parse(b"VMAP04E\0", limits).err(),
            Some(Error::InvalidLimits)
        );
        assert_eq!(
            Dtree::parse(b"VMAP_4.E", limits).err(),
            Some(Error::InvalidLimits)
        );
    }
}

#[test]
fn diagnostics_never_include_names_or_model_bytes() {
    assert_eq!(
        Error::MissingModel.to_string(),
        "gameobject_models_missing_model"
    );
    assert_eq!(
        Error::RawM2(crate::Error::Index).code(),
        "gameobject_models_raw_m2"
    );
    assert_eq!(
        Error::RawWmo(crate::wmo::Error::Shape).code(),
        "gameobject_models_raw_wmo"
    );
}
