// SPDX-License-Identifier: GPL-3.0-or-later
use super::*;
#[test]
fn camera_budget_is_downward_before_requested_read() {
    assert_eq!(camera_remaining(0).unwrap(), cameras::MAX_FILE_BYTES as u64);
    assert_eq!(camera_remaining(cameras::MAX_TOTAL_BYTES - 1).unwrap(), 1);
    for total in [
        cameras::MAX_TOTAL_BYTES,
        cameras::MAX_TOTAL_BYTES + 1,
        usize::MAX,
    ] {
        assert_eq!(
            camera_remaining(total).unwrap_err().code,
            "camera_policy_limit"
        );
    }
}
#[test]
fn v9_invalid_manual_plan_rejects_before_client_paths() {
    let mut p = Plan::parse(include_str!(
        "../../../../tests/contracts/fixtures/forever-product-v9/disabled.tsv"
    ))
    .unwrap();
    p.components = 1;
    assert_eq!(
        open(&p, Path::new("/nonexistent-authored-v9-client"))
            .err()
            .unwrap()
            .code,
        "product_plan_component_scope"
    );
}

#[test]
fn db2_v9_total_is_reduced_before_read() {
    assert_eq!(db2_remaining(0).unwrap(), DB2_FILE_BYTES);
    assert_eq!(db2_remaining(DB2_TOTAL_BYTES - 1).unwrap(), 1);
    for total in [DB2_TOTAL_BYTES, DB2_TOTAL_BYTES + 1, u64::MAX] {
        assert_eq!(
            db2_remaining(total).unwrap_err().code,
            "product_db2_total_limit"
        );
    }
}
