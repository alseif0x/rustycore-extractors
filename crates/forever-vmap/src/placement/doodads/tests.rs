// SPDX-License-Identifier: GPL-3.0-or-later
// Authored 2026-10-11: synthetic single-doodad arithmetic/policy regressions.
use super::*;
use crate::placement::{DirBin, encode};

const NAME: &[u8] = b"Model.m2";
const CONTEXT: Context = Context {
    map_id: 7,
    original_map_id: 7,
    global_wmo: false,
};

fn put_word(bytes: &mut [u8], at: usize, value: u32) {
    bytes[at..at + 4].copy_from_slice(&value.to_le_bytes());
}

fn parent_bytes(position: [f32; 3], rotation: [f32; 3]) -> [u8; 64] {
    let mut bytes = [0; 64];
    put_word(&mut bytes, 0, 99);
    put_word(&mut bytes, 4, 77);
    for (index, value) in position.into_iter().enumerate() {
        put_word(&mut bytes, 8 + index * 4, value.to_bits());
    }
    for (index, value) in rotation.into_iter().enumerate() {
        put_word(&mut bytes, 20 + index * 4, value.to_bits());
    }
    bytes
}

fn spawn(position: [f32; 3], quaternion: [f32; 4], scale: f32) -> DoodadSpawn {
    DoodadSpawn {
        name_index: 23,
        name_flags: 255,
        position_bits: position.map(f32::to_bits),
        rotation_bits: quaternion.map(f32::to_bits),
        scale_bits: scale.to_bits(),
        color: u32::MAX,
    }
}

fn identity_spawn() -> DoodadSpawn {
    spawn([0.0; 3], [0.0, 0.0, 0.0, 1.0], 1.0)
}

fn result(bytes: &[u8], spawn: &DoodadSpawn, context: Context) -> Placement<'static> {
    let parent = Modf::parse(bytes, Limits::default()).unwrap();
    project(&parent, spawn, 123, NAME, context, Limits::default()).unwrap()
}

fn near(actual: [f32; 3], expected: [f32; 3]) {
    for (a, e) in actual.into_iter().zip(expected) {
        assert!((a - e).abs() <= 0.0001, "{a} differs from {e}");
    }
}

#[test]
fn identity_projects_parent_zxy_and_unpermuted_child() {
    let bytes = parent_bytes([10.0, 20.0, 30.0], [0.0; 3]);
    let child = spawn([1.0, 2.0, 3.0], [0.0, 0.0, 0.0, 1.0], 2.0);
    let value = result(&bytes, &child, CONTEXT);
    assert_eq!(
        value.position_bits(),
        [31.0_f32, 12.0, 23.0].map(f32::to_bits)
    );
    assert_eq!(value.flags(), 0);
    assert_eq!(value.name_set(), 0);
    assert_eq!(value.bounds_bits(), None);
    assert_eq!(value.scale_bits(), 2.0_f32.to_bits());
}

#[test]
fn identity_euler_retains_expected_zero_signs() {
    let matrix = [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]];
    assert_eq!(
        euler_xyz(&matrix).map(f32::to_bits),
        [0x8000_0000, 0, 0x8000_0000]
    );
    let value = result(
        &parent_bytes([0.0; 3], [0.0; 3]),
        &identity_spawn(),
        CONTEXT,
    );
    assert_eq!(value.rotation_bits(), [0, 0x8000_0000, 0x8000_0000]);
}

#[test]
fn parent_rotation_y_is_world_z_rotation() {
    let child = spawn([1.0, 0.0, 0.0], [0.0, 0.0, 0.0, 1.0], 1.0);
    let value = result(&parent_bytes([0.0; 3], [0.0, 90.0, 0.0]), &child, CONTEXT);
    near(value.position_bits().map(f32::from_bits), [0.0, 1.0, 0.0]);
    near(value.rotation_bits().map(f32::from_bits), [0.0, 90.0, 0.0]);
}

#[test]
fn parent_rotation_x_is_world_y_rotation() {
    let child = spawn([1.0, 0.0, 0.0], [0.0, 0.0, 0.0, 1.0], 1.0);
    let value = result(&parent_bytes([0.0; 3], [90.0, 0.0, 0.0]), &child, CONTEXT);
    near(value.position_bits().map(f32::from_bits), [0.0, 0.0, -1.0]);
    near(value.rotation_bits().map(f32::from_bits), [90.0, 0.0, 0.0]);
}

#[test]
fn parent_rotation_z_is_world_x_rotation() {
    let child = spawn([0.0, 1.0, 0.0], [0.0, 0.0, 0.0, 1.0], 1.0);
    let value = result(&parent_bytes([0.0; 3], [0.0, 0.0, 90.0]), &child, CONTEXT);
    near(value.position_bits().map(f32::from_bits), [0.0, 0.0, 1.0]);
    near(value.rotation_bits().map(f32::from_bits), [0.0, 0.0, 90.0]);
}

#[test]
fn quaternion_is_normalized_not_assumed_unit() {
    let bytes = parent_bytes([0.0; 3], [0.0; 3]);
    let a = result(&bytes, &spawn([1.0; 3], [0.0, 0.0, 0.0, 1.0], 1.0), CONTEXT);
    for scalar in [2.0, -2.0, 0.5] {
        let b = result(
            &bytes,
            &spawn([1.0; 3], [0.0, 0.0, 0.0, scalar], 1.0),
            CONTEXT,
        );
        assert_eq!(a.position_bits(), b.position_bits());
        near(
            a.rotation_bits().map(f32::from_bits),
            b.rotation_bits().map(f32::from_bits),
        );
    }
}

#[test]
fn quarter_turn_quaternion_and_parent_use_q_times_r() {
    // Nonunit (1,0,0,1) normalizes to a quarter-turn about X; composing with
    // a parent Z quarter-turn gives Euler XYZ approximately (90,0,90).
    let child = spawn([1.0, 0.0, 0.0], [1.0, 0.0, 0.0, 1.0], 1.0);
    let value = result(&parent_bytes([0.0; 3], [0.0, 90.0, 0.0]), &child, CONTEXT);
    near(value.rotation_bits().map(f32::from_bits), [0.0, 90.0, 90.0]);
    // The child's quaternion does not rotate its translation.
    near(value.position_bits().map(f32::from_bits), [0.0, 1.0, 0.0]);
}

#[test]
fn finite_quaternion_norm_overflow_preserves_source_identity_result() {
    let bytes = parent_bytes([1.0, 2.0, 3.0], [0.0; 3]);
    let huge = spawn([4.0, 5.0, 6.0], [f32::MAX, 0.0, 0.0, 0.0], 1.0);
    let ordinary = spawn([4.0, 5.0, 6.0], [0.0, 0.0, 0.0, 1.0], 1.0);
    let a = result(&bytes, &huge, CONTEXT);
    let b = result(&bytes, &ordinary, CONTEXT);
    assert_eq!(a.position_bits(), b.position_bits());
    assert_eq!(a.rotation_bits(), b.rotation_bits());
}

#[test]
fn small_quaternion_with_positive_subnormal_norm_is_admitted() {
    let child = spawn([0.0; 3], [0.0, 0.0, 0.0, 1.0e-20], 1.0);
    let value = result(&parent_bytes([0.0; 3], [0.0; 3]), &child, CONTEXT);
    near(value.rotation_bits().map(f32::from_bits), [0.0; 3]);
}

#[test]
fn zero_and_underflowed_quaternion_norm_reject() {
    let bytes = parent_bytes([0.0; 3], [0.0; 3]);
    let parent = Modf::parse(&bytes, Limits::default()).unwrap();
    for quaternion in [[0.0; 4], [f32::from_bits(1); 4], [-0.0; 4]] {
        assert_eq!(
            project(
                &parent,
                &spawn([0.0; 3], quaternion, 1.0),
                1,
                NAME,
                CONTEXT,
                Limits::default()
            )
            .err(),
            Some(Error::NonFinite)
        );
    }
}

#[test]
fn parent_scale_path_flag_and_name_set_do_not_affect_child() {
    let a = parent_bytes([1.0, 2.0, 3.0], [0.0; 3]);
    let mut b = a;
    b[56..64].fill(255);
    let child = spawn([4.0, 5.0, 6.0], [0.0, 0.0, 0.0, 1.0], 0.25);
    let a = result(&a, &child, CONTEXT);
    let b = result(&b, &child, CONTEXT);
    assert_eq!(
        encode(&[a], Limits::default()).unwrap(),
        encode(&[b], Limits::default()).unwrap()
    );
    assert_eq!(b.flags(), 0);
    assert_eq!(b.name_set(), 0);
    assert_eq!(b.scale_bits(), 0.25_f32.to_bits());
}

#[test]
fn spawn_zero_negative_and_subnormal_scale_bits_survive() {
    let bytes = parent_bytes([0.0; 3], [0.0; 3]);
    for bits in [0, 0x8000_0000, (-2.0_f32).to_bits(), 1] {
        let mut child = identity_spawn();
        child.scale_bits = bits;
        assert_eq!(result(&bytes, &child, CONTEXT).scale_bits(), bits);
    }
}

#[test]
fn global_offset_is_added_before_child_translation() {
    // Cancellation makes the operation order observable: adding the shift
    // after parent+child would preserve its fractional part instead.
    let large = 16_777_216.0_f32;
    let child = spawn([-large, 2.0, 3.0], [0.0, 0.0, 0.0, 1.0], 1.0);
    let value = result(
        &parent_bytes([0.0, 0.0, large], [0.0; 3]),
        &child,
        Context {
            global_wmo: true,
            ..CONTEXT
        },
    );
    let offset = f32::from_bits(0x4685_5555);
    assert_eq!(
        value.position_bits(),
        [
            ((large + offset) - large).to_bits(),
            (offset + 2.0).to_bits(),
            3.0_f32.to_bits()
        ]
    );
    assert_ne!(value.position_bits()[0], offset.to_bits());
}

#[test]
fn global_addition_changes_parent_negative_zero_before_vector_sum() {
    // Every signed product in the identity third-row dot is -0, so omitting
    // the explicit global +0 operation would leave this emitted word negative.
    let bytes = parent_bytes([0.0, -0.0, 0.0], [0.0; 3]);
    let child = spawn([-0.0; 3], [0.0, 0.0, 0.0, 1.0], 1.0);
    assert_eq!(
        result(&bytes, &child, CONTEXT).position_bits()[2],
        0x8000_0000
    );
    let value = result(
        &bytes,
        &child,
        Context {
            global_wmo: true,
            ..CONTEXT
        },
    );
    assert_eq!(value.position_bits()[2], 0);
}

#[test]
fn gimbal_positive_and_negative_boundaries_use_fixed_half_pi() {
    for value in [1.0, 1.25, f32::MAX] {
        let m = [[0.0, 0.0, value], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]];
        assert_eq!(euler_xyz(&m).map(f32::to_bits), [0, HALF_PI.to_bits(), 0]);
    }
    for value in [-1.0, -1.25, -f32::MAX] {
        let m = [[0.0, 0.0, value], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]];
        assert_eq!(
            euler_xyz(&m).map(f32::to_bits),
            [0x8000_0000, (-HALF_PI).to_bits(), 0]
        );
    }
}

#[test]
fn interior_next_to_gimbal_keeps_both_atan2_angles() {
    for value in [f32::from_bits(0x3f7f_ffff), -f32::from_bits(0x3f7f_ffff)] {
        let m = [[1.0, -1.0, value], [0.0, 1.0, -1.0], [0.0, 0.0, 1.0]];
        let angles = euler_xyz(&m);
        assert_eq!(angles[0].to_bits(), 0x3f49_0fdb);
        assert_eq!(angles[2].to_bits(), 0x3f49_0fdb);
        assert!(angles[1].abs() < HALF_PI);
    }
}

#[test]
fn supplied_id_parent_flag_and_name_are_preserved() {
    let bytes = parent_bytes([0.0; 3], [0.0; 3]);
    let parent = Modf::parse(&bytes, Limits::default()).unwrap();
    let name = b"FILEabc.M2";
    let value = project(
        &parent,
        &identity_spawn(),
        u32::MAX,
        name,
        Context {
            map_id: 0,
            original_map_id: u32::MAX,
            global_wmo: false,
        },
        Limits::default(),
    )
    .unwrap();
    assert_eq!(value.id(), u32::MAX);
    assert_eq!(value.flags(), 2);
    assert_eq!(value.name().as_ptr(), name.as_ptr());
    assert_eq!(value.name(), name);
}

#[test]
fn projection_does_not_mutate_source_fields_or_parent_bytes() {
    let bytes = parent_bytes([10.0, 20.0, 30.0], [12.0, 23.0, 34.0]);
    let snapshot = bytes;
    let child = spawn([4.0, 5.0, 6.0], [1.0, 2.0, 3.0, 4.0], 0.5);
    let before = (child.position_bits, child.rotation_bits, child.scale_bits);
    let _ = result(&bytes, &child, CONTEXT);
    assert_eq!(bytes, snapshot);
    assert_eq!(
        (child.position_bits, child.rotation_bits, child.scale_bits),
        before
    );
}

#[test]
fn public_constructed_nonfinite_spawns_are_revalidated() {
    let bytes = parent_bytes([0.0; 3], [0.0; 3]);
    let parent = Modf::parse(&bytes, Limits::default()).unwrap();
    for bits in [
        f32::INFINITY.to_bits(),
        f32::NEG_INFINITY.to_bits(),
        f32::NAN.to_bits(),
    ] {
        for index in 0..8 {
            let mut child = identity_spawn();
            match index {
                0..=2 => child.position_bits[index] = bits,
                3..=6 => child.rotation_bits[index - 3] = bits,
                _ => child.scale_bits = bits,
            }
            assert_eq!(
                project(&parent, &child, 1, NAME, CONTEXT, Limits::default()).err(),
                Some(Error::NonFinite)
            );
        }
    }
}

#[test]
fn emitted_position_overflow_rejects() {
    let bytes = parent_bytes([0.0, 0.0, f32::MAX], [0.0; 3]);
    let parent = Modf::parse(&bytes, Limits::default()).unwrap();
    let child = spawn([f32::MAX, 0.0, 0.0], [0.0, 0.0, 0.0, 1.0], 1.0);
    assert_eq!(
        project(&parent, &child, 1, NAME, CONTEXT, Limits::default()).err(),
        Some(Error::NonFinite)
    );
}

#[test]
fn finite_angle_with_overflowed_radian_conversion_rejects() {
    let bytes = parent_bytes([0.0; 3], [f32::MAX, 0.0, 0.0]);
    let parent = Modf::parse(&bytes, Limits::default()).unwrap();
    assert_eq!(
        project(
            &parent,
            &identity_spawn(),
            1,
            NAME,
            CONTEXT,
            Limits::default()
        )
        .err(),
        Some(Error::NonFinite)
    );
}

#[test]
fn name_and_byte_record_limits_validate_before_math() {
    let bytes = parent_bytes([0.0; 3], [0.0; 3]);
    let parent = Modf::parse(&bytes, Limits::default()).unwrap();
    let child = spawn([0.0; 3], [0.0; 4], 1.0);
    for name in [b"".as_slice(), b"a\0b", &[255]] {
        assert_eq!(
            project(&parent, &child, 1, name, CONTEXT, Limits::default()).err(),
            Some(Error::Name)
        );
    }
    let child = identity_spawn();
    let exact = Limits {
        max_bytes: 38 + NAME.len(),
        max_records: 1,
        max_name_bytes: NAME.len(),
    };
    assert!(project(&parent, &child, 1, NAME, CONTEXT, exact).is_ok());
    for limits in [
        Limits {
            max_bytes: exact.max_bytes - 1,
            ..exact
        },
        Limits {
            max_records: 0,
            ..exact
        },
        Limits {
            max_name_bytes: NAME.len() - 1,
            ..exact
        },
    ] {
        assert_eq!(
            project(&parent, &child, 1, NAME, CONTEXT, limits).err(),
            Some(Error::Limit)
        );
    }
}

#[test]
fn enlarged_policy_limits_reject() {
    let bytes = parent_bytes([0.0; 3], [0.0; 3]);
    let parent = Modf::parse(&bytes, Limits::default()).unwrap();
    for limits in [
        Limits {
            max_bytes: super::super::MAX_BYTES + 1,
            ..Limits::default()
        },
        Limits {
            max_records: super::super::MAX_RECORDS + 1,
            ..Limits::default()
        },
        Limits {
            max_name_bytes: super::super::MAX_NAME_BYTES + 1,
            ..Limits::default()
        },
    ] {
        assert_eq!(
            project(&parent, &identity_spawn(), 1, NAME, CONTEXT, limits).err(),
            Some(Error::InvalidLimits)
        );
    }
}

#[test]
fn complete_dirbin_roundtrip_retains_projected_bits() {
    let value = result(
        &parent_bytes([1.0, 2.0, 3.0], [23.0, -41.0, 11.0]),
        &spawn([4.0, 5.0, 6.0], [1.0, -2.0, 3.0, 4.0], -0.0),
        CONTEXT,
    );
    let bytes = encode(&[value], Limits::default()).unwrap();
    assert_eq!(bytes.len(), 38 + NAME.len());
    let decoded = DirBin::parse(&bytes, Limits::default())
        .unwrap()
        .records()
        .next()
        .unwrap();
    assert_eq!(decoded.position_bits(), value.position_bits());
    assert_eq!(decoded.rotation_bits(), value.rotation_bits());
    assert_eq!(decoded.scale_bits(), 0x8000_0000);
    assert_eq!(decoded.bounds_bits(), None);
}
