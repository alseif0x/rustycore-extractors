// SPDX-License-Identifier: GPL-3.0-or-later
use super::*;
use crate::plan::{
    Decryption, OffMeshConnection,
    offmesh_tests::{connection, plan},
};

#[test]
fn v6_navigation_sources_preserve_original_axes_bits_order_and_duplicates() {
    let mut plan = plan();
    let first = connection();
    let second = OffMeshConnection {
        map_id: 7,
        direction: 1,
        ..first
    };
    plan.offmesh = Some(vec![first, second, first]);
    plan.validate().unwrap();
    let actual: Vec<_> = navigation_connections(&plan, false)
        .unwrap()
        .into_iter()
        .map(OffMeshConnection::from_source)
        .collect::<Result<_, _>>()
        .unwrap();
    assert_eq!(actual, [first, second, first]);
    assert_eq!(actual[0].from_bits[0], (-0.0f32).to_bits());
}

#[test]
fn v6_nonempty_connections_reject_an_empty_map_world() {
    let mut plan = plan();
    assert_eq!(
        navigation_connections(&plan, true).err().unwrap().code,
        "product_offmesh_empty_map"
    );
    plan.offmesh = Some(vec![]);
    assert!(navigation_connections(&plan, true).unwrap().is_empty());
    plan.offmesh = None;
    assert!(navigation_connections(&plan, true).unwrap().is_empty());
    assert!(navigation_connections(&plan, false).unwrap().is_empty());
}

#[test]
fn v6_invalid_manual_connections_reject_before_client_and_output_io() {
    for invalid in [
        OffMeshConnection {
            row: 64,
            ..connection()
        },
        OffMeshConnection {
            radius_bits: (-0.0f32).to_bits(),
            ..connection()
        },
        OffMeshConnection {
            to_bits: [0, f32::INFINITY.to_bits(), 0],
            ..connection()
        },
        OffMeshConnection {
            direction: 2,
            ..connection()
        },
    ] {
        let mut plan = plan();
        plan.offmesh.as_mut().unwrap().push(invalid);
        let client = Path::new("/nonexistent-authored-offmesh-client");
        let output = Path::new("/nonexistent-authored-offmesh-output");
        assert_eq!(
            open(&plan, client).err().unwrap().code,
            "product_plan_offmesh"
        );
        assert_eq!(
            extract(&plan, client, output, [0; 32], false)
                .unwrap_err()
                .code,
            "product_plan_offmesh"
        );
        assert_eq!(
            extract_disk(
                &plan,
                client,
                output,
                [0; 32],
                true,
                None,
                Path::new("/nonexistent-authored-offmesh-scratch")
            )
            .unwrap_err()
            .code,
            "product_plan_offmesh"
        );
    }
}

#[test]
fn v6_supplied_key_mode_keeps_key_admission_before_client_io() {
    let mut plan = plan();
    plan.decryption = Decryption::ProvidedSalsa20V1;
    assert_eq!(
        open(&plan, Path::new("/nonexistent-authored-offmesh-client"))
            .err()
            .unwrap()
            .code,
        "product_keys_required"
    );
}
