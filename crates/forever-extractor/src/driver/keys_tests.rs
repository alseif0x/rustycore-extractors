// SPDX-License-Identifier: GPL-3.0-or-later
// Authored 2026-10-11: plan/key capability admission before any client IO.
use super::*;
use crate::plan::Decryption;
fn plan() -> Plan {
    Plan::parse("forever-product\t1\nbuild\t70205\nlocale\tenUS\nindex\tcontiguous\nroot\theader-50893-v2\ncomponents\tgt\nterrain\tnone\npack\ttrue\nnavigation\tnone\nbig-base-unit\tfalse\nskip-liquids\tfalse\nmaximum-slope\t55\nnon-steep-slope\t55\n").unwrap()
}
#[test]
fn v2_without_keys_rejects_before_nonexistent_client_is_opened() {
    let mut plan = plan();
    plan.decryption = Decryption::ProvidedSalsa20V1;
    let error = open(&plan, Path::new("/nonexistent-authored-client"))
        .err()
        .unwrap();
    assert_eq!(error.code, "product_keys_required");
}
#[test]
fn v1_with_keys_rejects_before_nonexistent_client_is_opened() {
    let keys = forever_casc::keys::Keyring::parse(
        b"forever-tact-keys\t1\n0000000000000000\t00000000000000000000000000000000\n",
    )
    .unwrap();
    let error = open_with_keys(&plan(), Path::new("/nonexistent-authored-client"), keys)
        .err()
        .unwrap();
    assert_eq!(error.code, "product_keys_forbidden");
}
