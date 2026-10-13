// SPDX-License-Identifier: GPL-3.0-or-later
// Authored 2026-10-12: original checked aggregate physical disk admission.
use crate::{Error, plan::DiskBudget};
use forever_platform::linux::space::Available;
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
    // Six archive files, six owner directories and the private run directory.
    let scratch_inodes = 13;
    scratch = add(scratch, mul(scratch_unit, scratch_inodes)?)?;
    // Whole common package quota, bounded receipts (one <=1024-byte record per
    // artifact), one complete <=96MiB manifest and bounded state transitions.
    // Per-object fragments conservatively admit rounding and small directory
    // data, without promising a bound on all journal/CoW/filesystem metadata.
    let output_inodes = add(mul(FILES, 3)?, 32)?;
    let mut output = add(budget.output_bytes, mul(FILES, 1024)?)?;
    output = add(output, 96 * 1024 * 1024)?;
    output = add(output, HEADROOM)?;
    output = add(output, mul(output_unit, output_inodes)?)?;
    Ok([(scratch, scratch_inodes), (output, output_inodes)])
}
fn enough(observed: Available, (bytes, inodes): (u64, u64)) -> Result<(), Error> {
    if observed.bytes < bytes {
        return Err("product_disk_space".into());
    }
    if observed.inodes.is_some_and(|available| available < inodes) {
        return Err("product_disk_inodes".into());
    }
    Ok(())
}
pub(super) fn admit(
    budget: DiskBudget,
    scratch_device: u64,
    scratch: Available,
    output_device: u64,
    output: Available,
) -> Result<(), Error> {
    budget.validate().map_err(Error::from)?;
    let [scratch_need, output_need] =
        needs(budget, scratch.allocation_unit, output.allocation_unit)?;
    if scratch_device == output_device {
        let unit = scratch.allocation_unit.max(output.allocation_unit);
        let [scratch_need, output_need] = needs(budget, unit, unit)?;
        let common = Available {
            allocation_unit: scratch.allocation_unit.max(output.allocation_unit),
            bytes: scratch.bytes.min(output.bytes),
            inodes: match (scratch.inodes, output.inodes) {
                (Some(a), Some(b)) => Some(a.min(b)),
                (Some(a), None) | (None, Some(a)) => Some(a),
                (None, None) => None,
            },
        };
        enough(
            common,
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
    fn free(bytes: u64, inodes: Option<u64>) -> Available {
        Available {
            allocation_unit: 4096,
            bytes,
            inodes,
        }
    }
    #[test]
    fn same_filesystem_sums_both_endpoints() {
        let [a, b] = needs(budget(), 4096, 4096).unwrap();
        assert!(
            admit(
                budget(),
                1,
                free(a.0.max(b.0), None),
                1,
                free(a.0.max(b.0), None)
            )
            .is_err()
        );
        assert_eq!(
            admit(
                budget(),
                1,
                free(a.0 + b.0, Some(a.1 + b.1)),
                1,
                free(a.0 + b.0, Some(a.1 + b.1))
            ),
            Ok(())
        );
        assert_eq!(
            admit(
                budget(),
                1,
                free(a.0 + b.0, None),
                1,
                free(a.0 + b.0 - 1, None)
            )
            .unwrap_err()
            .code,
            "product_disk_space"
        );
    }
    #[test]
    fn separate_filesystems_are_admitted_independently() {
        let [a, b] = needs(budget(), 4096, 4096).unwrap();
        assert_eq!(
            admit(budget(), 1, free(a.0, Some(a.1)), 2, free(b.0, Some(b.1))),
            Ok(())
        );
        assert_eq!(
            admit(budget(), 1, free(a.0, Some(a.1 - 1)), 2, free(b.0, None))
                .unwrap_err()
                .code,
            "product_disk_inodes"
        );
        assert_eq!(
            admit(budget(), 1, free(a.0, None), 2, free(b.0, Some(b.1 - 1)))
                .unwrap_err()
                .code,
            "product_disk_inodes"
        );
    }
    #[test]
    fn same_filesystem_retains_known_inode_constraint() {
        let [a, b] = needs(budget(), 4096, 4096).unwrap();
        for (left, right) in [(Some(a.1 + b.1 - 1), None), (None, Some(a.1 + b.1 - 1))] {
            assert_eq!(
                admit(
                    budget(),
                    1,
                    free(a.0 + b.0, left),
                    1,
                    free(a.0 + b.0, right)
                )
                .unwrap_err()
                .code,
                "product_disk_inodes"
            );
        }
    }
    #[test]
    fn zero_allocation_units_and_arithmetic_overflow_reject() {
        assert!(needs(budget(), 0, 4096).is_err());
        assert!(needs(budget(), 4096, u64::MAX).is_err());
        assert!(needs(budget(), u64::MAX, 4096).is_err());
    }
    #[test]
    fn unknown_inode_counts_do_not_mean_zero_or_unlimited_bytes() {
        let [a, b] = needs(budget(), 4096, 4096).unwrap();
        assert_eq!(
            admit(budget(), 1, free(a.0, None), 2, free(b.0, None)),
            Ok(())
        );
        assert_eq!(
            admit(budget(), 1, free(0, None), 2, free(b.0, None))
                .unwrap_err()
                .code,
            "product_disk_space"
        );
    }
}
