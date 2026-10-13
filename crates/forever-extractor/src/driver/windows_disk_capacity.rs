// SPDX-License-Identifier: GPL-3.0-or-later
// Authored 2026-10-13: checked whole-product Windows physical space admission.
//! Pure arithmetic. Free-space observations are not reservations, quota proofs,
//! or bounds on all NTFS journal/metadata activity. Unknown inodes remain None.
use crate::{Error, plan::DiskBudget};

#[derive(Clone, Copy)]
pub(super) struct Available {
    pub volume: u32,
    pub allocation_unit: u64,
    pub bytes: u64,
    pub inodes: Option<u64>,
}
const FILES: u64 = 262_144;
const HEADROOM: u64 = 64 * 1024 * 1024;
fn add(a: u64, b: u64) -> Result<u64, Error> {
    a.checked_add(b).ok_or_else(|| "product_disk_limit".into())
}
fn mul(a: u64, b: u64) -> Result<u64, Error> {
    a.checked_mul(b).ok_or_else(|| "product_disk_limit".into())
}
fn needs(
    budget: DiskBudget,
    scratch_unit: u64,
    output_unit: u64,
) -> Result<[(u64, u64); 2], Error> {
    if scratch_unit == 0 || output_unit == 0 {
        return Err("product_disk_limit".into());
    }
    let mut scratch = HEADROOM;
    for bytes in [
        budget.raw_bytes,
        budget.wdt_bytes,
        budget.cache_bytes,
        budget.placement_bytes,
        budget.vmo_bytes,
        budget.map_bytes,
    ] {
        scratch = add(scratch, bytes)?;
    }
    // Six owner directories + six archives + six exclusive owner locks, plus
    // the fresh attempt namespace and its exclusive namespace lock.
    let scratch_objects = 20;
    scratch = add(scratch, mul(scratch_unit, scratch_objects)?)?;
    // Whole package payloads, bounded receipts, complete manifest, and control
    // transitions. Object allowance also admits per-file allocation rounding.
    let output_objects = add(mul(FILES, 3)?, 32)?;
    let mut output = add(budget.output_bytes, mul(FILES, 1024)?)?;
    output = add(output, 96 * 1024 * 1024)?;
    output = add(output, HEADROOM)?;
    output = add(output, mul(output_unit, output_objects)?)?;
    Ok([(scratch, scratch_objects), (output, output_objects)])
}
fn enough(observed: Available, (bytes, objects): (u64, u64)) -> Result<(), Error> {
    if observed.bytes < bytes {
        return Err("product_disk_space".into());
    }
    if observed.inodes.is_some_and(|available| available < objects) {
        return Err("product_disk_inodes".into());
    }
    Ok(())
}
pub(super) fn admit(
    budget: DiskBudget,
    scratch: Available,
    output: Available,
) -> Result<(), Error> {
    budget.validate().map_err(Error::from)?;
    let [scratch_need, output_need] =
        needs(budget, scratch.allocation_unit, output.allocation_unit)?;
    if scratch.volume == output.volume {
        let unit = scratch.allocation_unit.max(output.allocation_unit);
        let [scratch_need, output_need] = needs(budget, unit, unit)?;
        enough(
            Available {
                volume: scratch.volume,
                allocation_unit: unit,
                bytes: scratch.bytes.min(output.bytes),
                inodes: match (scratch.inodes, output.inodes) {
                    (Some(a), Some(b)) => Some(a.min(b)),
                    (Some(a), None) | (None, Some(a)) => Some(a),
                    (None, None) => None,
                },
            },
            (
                add(scratch_need.0, output_need.0)?,
                add(scratch_need.1, output_need.1)?,
            ),
        )
    } else {
        enough(scratch, scratch_need)?;
        enough(output, output_need)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn budget() -> DiskBudget {
        DiskBudget {
            raw_bytes: 32,
            wdt_bytes: 32,
            cache_bytes: 32,
            placement_bytes: 32,
            vmo_bytes: 32,
            map_bytes: 32,
            output_bytes: 1,
        }
    }
    fn available(volume: u32, bytes: u64, inodes: Option<u64>) -> Available {
        Available {
            volume,
            allocation_unit: 4096,
            bytes,
            inodes,
        }
    }
    #[test]
    fn same_volume_aggregates_both_endpoints_and_minimum_observation() {
        let [a, b] = needs(budget(), 4096, 4096).unwrap();
        assert_eq!(
            admit(
                budget(),
                available(1, a.0 + b.0, None),
                available(1, a.0 + b.0, None)
            ),
            Ok(())
        );
        assert_eq!(
            admit(
                budget(),
                available(1, a.0.max(b.0), None),
                available(1, a.0.max(b.0), None)
            )
            .unwrap_err()
            .code,
            "product_disk_space"
        );
        assert_eq!(
            admit(
                budget(),
                available(1, a.0 + b.0, None),
                available(1, a.0 + b.0 - 1, None)
            )
            .unwrap_err()
            .code,
            "product_disk_space"
        );
    }
    #[test]
    fn separate_volumes_keep_independent_bounds_and_known_inode_caps() {
        let [a, b] = needs(budget(), 4096, 4096).unwrap();
        assert_eq!(
            admit(
                budget(),
                available(1, a.0, Some(a.1)),
                available(2, b.0, Some(b.1))
            ),
            Ok(())
        );
        assert_eq!(
            admit(
                budget(),
                available(1, a.0 - 1, None),
                available(2, b.0, None)
            )
            .unwrap_err()
            .code,
            "product_disk_space"
        );
        assert_eq!(
            admit(
                budget(),
                available(1, a.0, None),
                available(2, b.0, Some(b.1 - 1))
            )
            .unwrap_err()
            .code,
            "product_disk_inodes"
        );
        assert_eq!(
            admit(
                budget(),
                available(1, a.0, Some(a.1 - 1)),
                available(2, b.0, None)
            )
            .unwrap_err()
            .code,
            "product_disk_inodes"
        );
    }
    #[test]
    fn same_volume_uses_largest_allocation_unit_and_retains_known_inode_cap() {
        let [a, b] = needs(budget(), 65536, 65536).unwrap();
        let mut output = available(1, a.0 + b.0, None);
        output.allocation_unit = 65536;
        assert_eq!(
            admit(budget(), available(1, a.0 + b.0, Some(a.1 + b.1)), output),
            Ok(())
        );
        assert_eq!(
            admit(
                budget(),
                available(1, a.0 + b.0, Some(a.1 + b.1 - 1)),
                output
            )
            .unwrap_err()
            .code,
            "product_disk_inodes"
        );
    }
    #[test]
    fn output_budget_is_whole_package_and_every_spool_is_charged() {
        let [a, b] = needs(budget(), 4096, 4096).unwrap();
        let mut changed = budget();
        changed.output_bytes += 1;
        assert_eq!(
            admit(changed, available(1, a.0, None), available(2, b.0, None))
                .unwrap_err()
                .code,
            "product_disk_space"
        );
        for at in 0..6 {
            let mut changed = budget();
            *match at {
                0 => &mut changed.raw_bytes,
                1 => &mut changed.wdt_bytes,
                2 => &mut changed.cache_bytes,
                3 => &mut changed.placement_bytes,
                4 => &mut changed.vmo_bytes,
                _ => &mut changed.map_bytes,
            } += 1;
            assert_eq!(
                admit(changed, available(1, a.0, None), available(2, b.0, None))
                    .unwrap_err()
                    .code,
                "product_disk_space"
            );
        }
    }
    #[test]
    fn zero_units_overflow_and_invalid_budget_reject_without_io() {
        assert!(needs(budget(), 0, 4096).is_err());
        assert!(needs(budget(), 4096, u64::MAX).is_err());
        assert!(needs(budget(), u64::MAX, 4096).is_err());
        let mut invalid = budget();
        invalid.raw_bytes = 0;
        assert_eq!(
            admit(invalid, available(1, 0, None), available(2, 0, None))
                .unwrap_err()
                .code,
            "product_plan_disk_budget"
        );
    }
}
