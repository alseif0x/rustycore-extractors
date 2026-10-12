// SPDX-License-Identifier: GPL-3.0-or-later
// Authored 2026-10-11: original synthetic row-wise/owned list comparisons.
use super::*;
use crate::gameobject_models::{self as old, Dtree, ResolvedRow};
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

fn meta<'a>(rows: &[ResolvedRow<'a>]) -> Vec<Metadata<'a>> {
    rows.iter()
        .map(|r| Metadata {
            display_id: r.display_id,
            name: r.name,
            model: r.model.map(|m| match m {
                RawModel::M2(b) => ModelInfo {
                    class: Class::M2,
                    raw_bytes: b.len(),
                },
                RawModel::Wmo(b) => ModelInfo {
                    class: Class::Wmo,
                    raw_bytes: b.len(),
                },
            }),
        })
        .collect()
}
fn run(rows: &[ResolvedRow<'_>], limits: Limits) -> Result<Lists, Error> {
    let metadata = meta(rows);
    let plan = Plan::preflight(&metadata, limits)?;
    let mut b = Builder::new(plan, &metadata, limits)?;
    for r in rows {
        b.push(r.model.ok_or(CodecError::MissingModel)?)?
    }
    b.finish()
}
fn row<'a>(id: u32, name: &'a [u8], raw: &'a [u8]) -> ResolvedRow<'a> {
    ResolvedRow {
        display_id: id,
        name,
        model: Some(RawModel::M2(raw)),
    }
}
fn compare(rows: &[ResolvedRow<'_>]) {
    let a = old::build(rows, Limits::default()).unwrap();
    let b = run(rows, Limits::default()).unwrap();
    assert_eq!(a.temp_bytes(), b.temp_bytes());
    assert_eq!(a.dtree_bytes(), b.dtree_bytes());
    let x = a.stats();
    let y = b.stats();
    assert_eq!(
        (
            x.rows,
            x.m2_rows,
            x.wmo_rows,
            x.vertex_visits,
            x.raw_visit_bytes,
            x.temp_bytes,
            x.dtree_bytes,
            x.resident_bytes
        ),
        (
            y.rows,
            y.m2_rows,
            y.wmo_rows,
            y.vertex_visits,
            y.raw_visit_bytes,
            y.temp_bytes,
            y.dtree_bytes,
            y.resident_bytes
        )
    );
}
#[test]
fn exact_m2_wire_and_repeated_physical_rows_equal_owned() {
    let a = m2(&triangle());
    let b = m2(&[[0; 3]; 3]);
    compare(&[
        row(9, b"same", &a),
        row(0, b"second", &b),
        row(9, b"same", &b),
        row(u32::MAX, b"same", &a),
    ]);
}
#[test]
fn wmo_multigroup_unreferenced_vertices_match_owned() {
    let mut extra = triangle();
    extra.push([40f32, -50., 60.].map(f32::to_bits));
    let raw = wmo(
        &[group(&[]), group(&triangle()), group(&extra), group(&[])],
        2,
    );
    compare(&[ResolvedRow {
        display_id: 1,
        name: b"w",
        model: Some(RawModel::Wmo(&raw)),
    }]);
}
#[test]
fn signedzero_last_equal_vertex_wins_exactly() {
    for bits in [0, 0x80000000] {
        let raw = wmo(
            &[group(&[[bits ^ 0x80000000; 3]; 3]), group(&[[bits; 3]; 3])],
            2,
        );
        let out = run(
            &[ResolvedRow {
                display_id: 0,
                name: b"zero",
                model: Some(RawModel::Wmo(&raw)),
            }],
            Limits::default(),
        )
        .unwrap();
        assert_eq!(
            Dtree::parse(out.dtree_bytes(), Limits::default())
                .unwrap()
                .rows()
                .next()
                .unwrap()
                .bounds_bits(),
            [bits; 6]
        );
        compare(&[ResolvedRow {
            display_id: 0,
            name: b"zero",
            model: Some(RawModel::Wmo(&raw)),
        }]);
    }
}
#[test]
fn empty_rows_need_two_headers_and_no_plan_records() {
    let limits = Limits {
        max_memory_bytes: 16,
        max_rows: 0,
        max_name_bytes: 0,
        max_vertex_visits: 0,
        max_raw_visit_bytes: 0,
        max_raw_bytes: 0,
        ..Limits::default()
    };
    let p = Plan::preflight(&[], limits).unwrap();
    assert_eq!(p.planned_bytes(), 16);
    assert_eq!(p.rows(), 0);
    assert_eq!(p.raw_visit_bytes(), 0);
    let out = Builder::new(p, &[], limits).unwrap().finish().unwrap();
    assert_eq!(out.temp_bytes(), old::TEMP_MAGIC);
    assert_eq!(out.dtree_bytes(), old::DTREE_MAGIC);
    assert_eq!(out.stats().resident_bytes, 16);
    assert_eq!(out.stats().peak_bytes, 16);
    assert_eq!(
        Plan::preflight(
            &[],
            Limits {
                max_memory_bytes: 15,
                ..limits
            }
        )
        .err(),
        Some(Error::Codec(CodecError::Limit))
    );
    assert_eq!(
        Plan::preflight(
            &[],
            Limits {
                max_list_bytes: 7,
                ..limits
            }
        )
        .err(),
        Some(Error::Codec(CodecError::Limit))
    );
}
#[test]
fn preflight_all_names_and_dependencies_precedes_raw_and_memory() {
    let bad = row(1, b"ok", b"broken");
    for later in [
        ResolvedRow {
            display_id: 2,
            name: b"missing",
            model: None,
        },
        row(2, b"", b""),
    ] {
        let err = old::build(
            &[bad, later],
            Limits {
                max_memory_bytes: 0,
                ..Limits::default()
            },
        )
        .err()
        .unwrap();
        assert_eq!(
            run(
                &[bad, later],
                Limits {
                    max_memory_bytes: 0,
                    ..Limits::default()
                }
            )
            .err(),
            Some(Error::Codec(err))
        );
    }
}
#[test]
fn all_raw_lengths_list_lengths_and_cumulative_visits_are_preflighted() {
    let raw = m2(&triangle());
    let rows = [row(1, b"a", &raw), row(2, b"b", &raw)];
    for limits in [
        Limits {
            max_rows: 1,
            ..Limits::default()
        },
        Limits {
            max_raw_bytes: raw.len() - 1,
            ..Limits::default()
        },
        Limits {
            max_raw_visit_bytes: 2 * raw.len() as u64 - 1,
            ..Limits::default()
        },
        Limits {
            max_list_bytes: 8,
            ..Limits::default()
        },
    ] {
        assert_eq!(
            Plan::preflight(&meta(&rows), limits).err(),
            Some(Error::Codec(CodecError::Limit))
        );
    }
}
#[test]
fn full_output_and_plan_capacities_admitted_before_allocation() {
    let raw = m2(&triangle());
    let rows = [row(1, b"abc", &raw)];
    let metadata = meta(&rows);
    let p = Plan::preflight(&metadata, Limits::default()).unwrap();
    let exact = Limits {
        max_memory_bytes: p.planned_bytes(),
        ..Limits::default()
    };
    let mut b = Builder::new(p, &metadata, exact).unwrap();
    assert_eq!(b.resident_bytes().unwrap(), p.planned_bytes());
    b.push(RawModel::M2(&raw)).unwrap();
    assert!(b.finish().is_ok());
    assert_eq!(
        Plan::preflight(
            &metadata,
            Limits {
                max_memory_bytes: p.planned_bytes() - 1,
                ..exact
            }
        )
        .err(),
        Some(Error::Codec(CodecError::Limit))
    );
}
#[test]
fn altered_plan_rejected_before_build() {
    let raw = m2(&triangle());
    let metadata = meta(&[row(1, b"a", &raw)]);
    let mut p = Plan::preflight(&metadata, Limits::default()).unwrap();
    p.rows += 1;
    assert_eq!(
        Builder::new(p, &metadata, Limits::default()).err(),
        Some(Error::State)
    );
}
#[test]
fn callback_class_and_exact_length_must_match_plan() {
    let raw = m2(&triangle());
    let metadata = meta(&[row(1, b"a", &raw)]);
    let p = Plan::preflight(&metadata, Limits::default()).unwrap();
    for model in [RawModel::Wmo(&raw), RawModel::M2(&raw[..raw.len() - 1])] {
        let mut b = Builder::new(p, &metadata, Limits::default()).unwrap();
        assert_eq!(b.push(model), Err(Error::State));
        assert_eq!(b.finish().err(), Some(Error::State));
    }
}
#[test]
fn missing_and_extra_rows_cannot_finish_or_replay() {
    let raw = m2(&triangle());
    let metadata = meta(&[row(1, b"a", &raw)]);
    let p = Plan::preflight(&metadata, Limits::default()).unwrap();
    assert_eq!(
        Builder::new(p, &metadata, Limits::default())
            .unwrap()
            .finish()
            .err(),
        Some(Error::State)
    );
    let mut b = Builder::new(p, &metadata, Limits::default()).unwrap();
    b.push(RawModel::M2(&raw)).unwrap();
    assert_eq!(b.push(RawModel::M2(&raw)), Err(Error::State));
    assert_eq!(b.finish().err(), Some(Error::State));
}
#[test]
fn cumulative_vertex_quota_checked_during_each_parse_is_sticky() {
    let raw = m2(&triangle());
    let rows = [row(1, b"a", &raw), row(2, b"a", &raw)];
    let metadata = meta(&rows);
    let limits = Limits {
        max_vertex_visits: 5,
        ..Limits::default()
    };
    let p = Plan::preflight(&metadata, limits).unwrap();
    let mut b = Builder::new(p, &metadata, limits).unwrap();
    b.push(RawModel::M2(&raw)).unwrap();
    let e = b.push(RawModel::M2(&raw)).unwrap_err();
    assert_eq!(e, Error::Codec(CodecError::Limit));
    assert_eq!(b.push(RawModel::M2(&raw)), Err(e));
    assert_eq!(b.finish().err(), Some(e));
}
#[test]
fn all_raw_truncations_and_nonfinite_inputs_match_owned_errors() {
    let raw = m2(&triangle());
    for cut in 0..raw.len() {
        let rows = [row(1, b"a", &raw[..cut])];
        assert_eq!(
            run(&rows, Limits::default()).err(),
            old::build(&rows, Limits::default()).err().map(Error::Codec)
        );
    }
    let mut bad = raw;
    set_word(&mut bad, 32, f32::INFINITY.to_bits());
    let rows = [row(1, b"a", &bad)];
    assert_eq!(
        run(&rows, Limits::default()).err(),
        old::build(&rows, Limits::default()).err().map(Error::Codec)
    );
}
#[test]
fn late_malformed_raw_never_exposes_first_row_lists() {
    let raw = m2(&triangle());
    let rows = [row(1, b"a", &raw), row(2, b"bad", b"broken")];
    assert!(matches!(
        run(&rows, Limits::default()),
        Err(Error::Codec(CodecError::RawM2(_)))
    ));
}
#[test]
fn malformed_wmo_and_vertex_empty_are_terminal() {
    let bad = b"broken";
    let rows = [ResolvedRow {
        display_id: 1,
        name: b"a",
        model: Some(RawModel::Wmo(bad)),
    }];
    assert_eq!(
        run(&rows, Limits::default()).err(),
        old::build(&rows, Limits::default()).err().map(Error::Codec)
    );
    let raw = wmo(&[group(&[])], 0);
    let rows = [ResolvedRow {
        display_id: 1,
        name: b"a",
        model: Some(RawModel::Wmo(&raw)),
    }];
    assert_eq!(
        run(&rows, Limits::default()).err(),
        old::build(&rows, Limits::default()).err().map(Error::Codec)
    );
}
#[test]
fn names_are_exact_opaque_ascii_under_unchanged_limits() {
    let raw = m2(&triangle());
    for name in [&b"../a\\B.m2"[..], &vec![b'a'; 499], b"\x01\x7f"] {
        compare(&[row(1, name, &raw)]);
    }
    for name in [&b""[..], b"a\0b", b"\x80", &vec![b'a'; 500]] {
        let rows = [row(1, name, &raw)];
        assert_eq!(
            run(&rows, Limits::default()).err(),
            old::build(&rows, Limits::default()).err().map(Error::Codec)
        );
    }
}
#[test]
fn old_codec_error_is_still_exhaustively_matchable() {
    fn exact(e: CodecError) -> u8 {
        match e {
            CodecError::InvalidLimits => 0,
            CodecError::Limit => 1,
            CodecError::Allocation => 2,
            CodecError::Framing => 3,
            CodecError::Name => 4,
            CodecError::NonFinite => 5,
            CodecError::InvertedBounds => 6,
            CodecError::MissingModel => 7,
            CodecError::EmptyVertices => 8,
            CodecError::RawM2(_) => 9,
            CodecError::RawWmo(_) => 10,
        }
    }
    assert_eq!(exact(CodecError::Name), 4);
}
