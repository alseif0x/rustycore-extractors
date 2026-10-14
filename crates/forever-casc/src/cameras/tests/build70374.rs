// SPDX-License-Identifier: GPL-3.0-or-later
//! Authored current-catalog source bytes; no client or camera assets.
use super::*;
fn target(ids: &[u32], copies: &[(u32, u32)]) -> Vec<u8> {
    let mut bytes = source(ids, copies);
    put32(&mut bytes, 152, CameraContract::TABLE_HASH);
    bytes
}
#[test]
fn new_catalog_direct_values_keep_zero_max_and_pre_copy_set() {
    let bytes = target(&[u32::MAX, 0, 11, 11], &[(2, 1), (5, 2)]);
    let p = discover_70374(&bytes).unwrap();
    assert_eq!(p.file_ids(), &[0, 11, u32::MAX].into_iter().collect());
    assert_eq!(p.facts().direct_records, 4);
    assert_eq!(p.facts().unique_file_ids, 3);
    assert_eq!(p.facts().file_id_set_sha256, file_id_set_hash(p.file_ids()));
}
#[test]
fn new_catalog_requires_exact_full_source_schema_and_copy_tail() {
    let original = target(&[11], &[]);
    for field in [152, 156] {
        let mut bad = original.clone();
        put32(&mut bad, field, 0);
        assert!(discover_70374(&bad).is_err());
    }
    let mut trailing = original;
    trailing.push(0);
    assert!(discover_70374(&trailing).is_err());
    assert!(discover_70374(&target(&[11], &[(1, 1)])).is_err());
}
#[test]
fn duplicate_direct_row_ids_fail_but_duplicate_file_ids_collapse() {
    let mut bytes = target(&[11, 11], &[]);
    let ids = bytes.len() - 8;
    put32(&mut bytes, ids, 2);
    put32(&mut bytes, ids + 4, 2);
    assert_eq!(discover_70374(&bytes).err(), Some("invalid_record_ids"));
    assert_eq!(
        discover_70374(&target(&[11, 11], &[]))
            .unwrap()
            .file_ids()
            .len(),
        1
    );
}
#[test]
fn exact_discovery_limits_and_empty_source_stay_bounded() {
    let ids = (0..MAX_FILES as u32).collect::<Vec<_>>();
    assert_eq!(
        discover_70374(&target(&ids, &[])).unwrap().file_ids().len(),
        MAX_FILES
    );
    let ids = (0..=MAX_FILES as u32).collect::<Vec<_>>();
    assert_eq!(
        discover_70374(&target(&ids, &[])).err(),
        Some("camera_policy_limit")
    );
    assert_eq!(
        discover_70374(&vec![0; MAX_SOURCE_BYTES + 1]).err(),
        Some("camera_policy_limit")
    );
}
#[test]
fn standalone_contracts_do_not_admit_70374() {
    assert!(CameraContract::Legacy.admit_build(70374).is_err());
    assert!(CameraContract::Build70338V1.admit_build(70374).is_err());
    assert!(
        crate::gametables::contract::CatalogContract::Legacy
            .admit_build(70374)
            .is_err()
    );
    assert!(
        crate::gametables::contract::CatalogContract::Build70338V1
            .admit_build(70374)
            .is_err()
    );
}
