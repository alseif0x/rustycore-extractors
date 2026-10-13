// SPDX-License-Identifier: GPL-3.0-or-later
use super::*;
use crate::plan::offmesh_tests::{connection, plan};
use forever_navigation::{mmap, offmesh};

fn global(map: u32, sources: &[offmesh::Source]) -> Vec<u8> {
    mmap::encode_global(
        map,
        mmap::Params {
            origin: [0.0; 3],
            tile_width: 1.0,
            tile_height: 1.0,
            max_tiles: 1,
            max_polys: 1,
        },
        sources,
        Default::default(),
    )
    .unwrap()
}

fn inspect(plan: &Plan, map: u32, sources: &[offmesh::Source]) -> Result<(), Error> {
    let bytes = global(map, sources);
    Facts::default().inspect(
        plan,
        &Artifact {
            component: Component::Navigation,
            path: mmap::global_path(map),
            bytes: bytes.len() as u64,
            sha256: integrity::digest(&bytes),
        },
        &bytes,
    )
}

#[test]
fn v6_global_sources_match_exact_map_subsets_order_and_duplicates() {
    let first = connection();
    let other_map = OffMeshConnection { map_id: 7, ..first };
    let second = OffMeshConnection {
        direction: 1,
        flags: 1,
        ..first
    };
    let mut plan = plan();
    plan.offmesh = Some(vec![first, other_map, first, second]);
    plan.validate().unwrap();
    assert!(
        inspect(
            &plan,
            42,
            &[first.source(), first.source(), second.source()]
        )
        .is_ok()
    );
    assert!(inspect(&plan, 7, &[other_map.source()]).is_ok());
    assert!(inspect(&plan, 8, &[]).is_ok());
}

#[test]
fn v6_global_rejects_missing_extra_reordered_or_deduplicated_connections() {
    let first = connection();
    let second = OffMeshConnection {
        direction: 1,
        ..first
    };
    let mut plan = plan();
    plan.offmesh = Some(vec![first, first, second]);
    for sources in [
        vec![],
        vec![first.source(), second.source()],
        vec![first.source(), second.source(), first.source()],
        vec![
            first.source(),
            first.source(),
            second.source(),
            second.source(),
        ],
    ] {
        assert_eq!(
            inspect(&plan, 42, &sources).unwrap_err().code,
            "product_offmesh_mismatch"
        );
    }
}

#[test]
fn v6_global_comparison_checks_all_scalar_bits_and_signed_zero() {
    let original = connection();
    let plan = plan();
    for field in 0..12 {
        let mut changed = original;
        match field {
            0 => changed.row += 1,
            1 => changed.column += 1,
            2..=4 => changed.from_bits[field - 2] ^= 1,
            5..=7 => changed.to_bits[field - 5] ^= 1,
            8 => changed.radius_bits ^= 1,
            9 => changed.direction ^= 1,
            10 => changed.area ^= 1,
            _ => changed.flags ^= 1,
        }
        assert_eq!(
            inspect(&plan, 42, &[changed.source()]).unwrap_err().code,
            "product_offmesh_mismatch",
            "field {field}"
        );
    }
    let changed = OffMeshConnection {
        from_bits: [0, original.from_bits[1], original.from_bits[2]],
        ..original
    };
    assert_eq!(
        inspect(&plan, 42, &[changed.source()]).unwrap_err().code,
        "product_offmesh_mismatch"
    );
}

#[test]
fn v6_global_invalid_raw_records_reject_without_lossy_coordinate_conversion() {
    let plan = plan();
    for field in 0..5 {
        let mut source = connection().source();
        match field {
            0 => source.row = 256,
            1 => source.column = u32::MAX,
            2 => source.radius = f32::NAN,
            3 => source.connection_flags = 2,
            _ => source.area = 64,
        }
        assert_eq!(
            inspect(&plan, 42, &[source]).unwrap_err().code,
            "product_offmesh_mismatch"
        );
    }
}

#[test]
fn v6_explicit_empty_global_accepts_no_records_and_rejects_any_record() {
    let mut plan = plan();
    plan.offmesh = Some(vec![]);
    assert!(inspect(&plan, 42, &[]).is_ok());
    assert_eq!(
        inspect(&plan, 42, &[connection().source()])
            .unwrap_err()
            .code,
        "product_offmesh_mismatch"
    );
}

#[test]
fn legacy_global_rejection_is_unchanged() {
    let mut plan = plan();
    plan.offmesh = None;
    assert!(inspect(&plan, 42, &[]).is_ok());
    assert_eq!(
        inspect(&plan, 42, &[connection().source()])
            .unwrap_err()
            .code,
        "product_offmesh_unsupported"
    );
}

#[test]
fn v6_invalid_plan_rejects_before_output_is_opened() {
    let mut plan = plan();
    plan.offmesh.as_mut().unwrap()[0].direction = 2;
    assert_eq!(
        verify(
            &plan,
            Path::new("/nonexistent-authored-offmesh-package"),
            None
        )
        .unwrap_err()
        .code,
        "product_plan_offmesh"
    );
}
