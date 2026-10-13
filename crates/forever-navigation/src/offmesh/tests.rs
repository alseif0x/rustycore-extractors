// SPDX-License-Identifier: GPL-3.0-or-later
// Authored 2026-10-11: synthetic off-mesh projection checks.
use super::*;
fn target() -> Target {
    Target {
        map_id: 7,
        row: 24,
        column: 39,
    }
}
fn source() -> Source {
    Source {
        map_id: 7,
        row: 24,
        column: 39,
        from: [1.0, 2.0, 3.0],
        to: [4.0, 5.0, 6.0],
        radius: 1.5,
        connection_flags: 255,
        area: 255,
        flags: 65535,
    }
}
#[test]
fn reorder_without_mirroring_and_preserve_opaque_flags() {
    let c = select(target(), &[source()], Limits::default()).unwrap();
    let c = c.values()[0];
    assert_eq!(c.endpoints(), [[2.0, 3.0, 1.0], [5.0, 6.0, 4.0]]);
    assert_eq!(c.radius(), 1.5);
    assert_eq!((c.direction(), c.area(), c.flags()), (1, 255, 65535));
}
#[test]
fn exact_selection_keeps_order_and_duplicates() {
    let s = source();
    let input = [
        Source { map_id: 8, ..s },
        s,
        Source {
            row: 39,
            column: 24,
            ..s
        },
        Source { radius: 2.5, ..s },
        s,
    ];
    let c = select(target(), &input, Limits::default()).unwrap();
    assert_eq!(
        c.values()
            .iter()
            .map(Connection::radius)
            .collect::<Vec<_>>(),
        [1.5, 2.5, 1.5]
    );
}
#[test]
fn all_direction_bits_are_masked_to_low_bit() {
    let s = source();
    for f in 0..=255 {
        let c = select(
            target(),
            &[Source {
                connection_flags: f,
                ..s
            }],
            Limits::default(),
        )
        .unwrap();
        assert_eq!(c.values()[0].direction(), f & 1);
    }
}
#[test]
fn signed_zero_negative_radius_and_full_ids_survive() {
    let mut s = source();
    s.from = [-0.0, 0.0, -0.0];
    s.radius = -0.0;
    s.map_id = u32::MAX;
    let t = Target {
        map_id: u32::MAX,
        ..target()
    };
    let c = select(t, &[s], Limits::default()).unwrap();
    assert_eq!(
        c.values()[0].endpoints()[0].map(f32::to_bits),
        [0, 0x80000000, 0x80000000]
    );
    assert_eq!(c.values()[0].radius().to_bits(), 0x80000000);
    s.radius = -2.5;
    assert_eq!(
        select(t, &[s], Limits::default()).unwrap().values()[0].radius(),
        -2.5
    );
}
#[test]
fn selected_nonfinite_values_fail_but_unselected_records_are_ignored() {
    let s = source();
    for bad in [
        Source {
            radius: f32::NAN,
            ..s
        },
        Source {
            from: [f32::INFINITY, 0.0, 0.0],
            ..s
        },
        Source {
            to: [0.0, 0.0, f32::NEG_INFINITY],
            ..s
        },
    ] {
        assert!(matches!(
            select(target(), &[s, bad], Limits::default()),
            Err(Error::NonFinite)
        ));
        assert_eq!(
            select(
                target(),
                &[s, Source { map_id: 0, ..bad }],
                Limits::default()
            )
            .unwrap()
            .values()
            .len(),
            1
        );
    }
}
#[test]
fn policy_coordinates_empty_input_and_exact_memory_boundary() {
    assert!(
        select(
            target(),
            &[],
            Limits {
                input_records: 0,
                output_records: 0,
                heap_bytes: 0
            }
        )
        .unwrap()
        .values()
        .is_empty()
    );
    assert!(matches!(
        select(
            Target {
                row: 64,
                ..target()
            },
            &[],
            Limits::default()
        ),
        Err(Error::Coordinates)
    ));
    for l in [
        Limits {
            input_records: 0,
            ..Default::default()
        },
        Limits {
            output_records: 0,
            ..Default::default()
        },
        Limits {
            heap_bytes: 31,
            ..Default::default()
        },
        Limits {
            input_records: 65537,
            ..Default::default()
        },
        Limits {
            output_records: 65537,
            ..Default::default()
        },
        Limits {
            heap_bytes: usize::MAX,
            ..Default::default()
        },
    ] {
        assert!(matches!(
            select(target(), &[source()], l),
            Err(Error::Limit)
        ));
    }
    let c = select(
        target(),
        &[source()],
        Limits {
            input_records: 1,
            output_records: 1,
            heap_bytes: 32,
        },
    )
    .unwrap();
    assert_eq!(c.capacity_bytes(), 32);
}
#[test]
fn record_ceiling_accepts_exact_maximum() {
    let c = select(target(), &vec![source(); 65536], Limits::default()).unwrap();
    assert_eq!(c.values().len(), 65536);
    assert_eq!(c.capacity_bytes(), 2 * 1024 * 1024);
    assert!(matches!(
        select(target(), &vec![source(); 65537], Limits::default()),
        Err(Error::Limit)
    ));
}
