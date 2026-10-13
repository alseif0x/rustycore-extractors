// SPDX-License-Identifier: GPL-3.0-or-later
// Authored 2026-10-12: explicit cumulative collision disk admission budgets.
use super::field;

/// Storage is an explicit plan capability, independent of supplied decryption.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum CollisionStorage {
    #[default]
    Resident,
    LeasedDiskV1(DiskBudget),
}

/// Cumulative reservations, not free-space measurements or memory/RSS limits.
/// These values only reduce the fixed disk-world profile ceilings. Paths,
/// source identities and key material never enter this public plan.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DiskBudget {
    pub raw_bytes: u64,
    pub wdt_bytes: u64,
    pub cache_bytes: u64,
    pub placement_bytes: u64,
    pub vmo_bytes: u64,
    pub map_bytes: u64,
    pub output_bytes: u64,
}

impl DiskBudget {
    pub const MIN_RECORD_BYTES: u64 = 32;
    pub const MAX_RECORD_BYTES: u64 = 128 * 1024 * 1024 * 1024;
    pub const MAX_OUTPUT_BYTES: u64 = 64 * 1024 * 1024 * 1024;

    /// Validate manually constructed budgets before filesystem access.
    pub fn validate(self) -> Result<(), &'static str> {
        for bytes in [
            self.raw_bytes,
            self.wdt_bytes,
            self.cache_bytes,
            self.placement_bytes,
            self.vmo_bytes,
            self.map_bytes,
        ] {
            if !(Self::MIN_RECORD_BYTES..=Self::MAX_RECORD_BYTES).contains(&bytes) {
                return Err("product_plan_disk_budget");
            }
        }
        if !(1..=Self::MAX_OUTPUT_BYTES).contains(&self.output_bytes) {
            return Err("product_plan_disk_budget");
        }
        Ok(())
    }
}

impl CollisionStorage {
    pub(super) fn parse<'a>(
        lines: &mut impl Iterator<Item = &'a str>,
    ) -> Result<Self, &'static str> {
        if field(lines, "collision-storage")? != "linux-leased-append-disk-world-v1" {
            return Err("product_plan_collision_storage");
        }
        let budget = DiskBudget {
            raw_bytes: decimal(field(lines, "collision-raw-bytes")?)?,
            wdt_bytes: decimal(field(lines, "collision-wdt-bytes")?)?,
            cache_bytes: decimal(field(lines, "collision-cache-bytes")?)?,
            placement_bytes: decimal(field(lines, "collision-placement-bytes")?)?,
            vmo_bytes: decimal(field(lines, "collision-vmo-bytes")?)?,
            map_bytes: decimal(field(lines, "collision-map-bytes")?)?,
            output_bytes: decimal(field(lines, "collision-output-bytes")?)?,
        };
        budget.validate()?;
        Ok(Self::LeasedDiskV1(budget))
    }

    pub(super) fn render(self) -> String {
        let Self::LeasedDiskV1(budget) = self else {
            return String::new();
        };
        format!(
            "collision-storage\tlinux-leased-append-disk-world-v1\ncollision-raw-bytes\t{}\ncollision-wdt-bytes\t{}\ncollision-cache-bytes\t{}\ncollision-placement-bytes\t{}\ncollision-vmo-bytes\t{}\ncollision-map-bytes\t{}\ncollision-output-bytes\t{}\n",
            budget.raw_bytes,
            budget.wdt_bytes,
            budget.cache_bytes,
            budget.placement_bytes,
            budget.vmo_bytes,
            budget.map_bytes,
            budget.output_bytes,
        )
    }
}

fn decimal(value: &str) -> Result<u64, &'static str> {
    if value.is_empty()
        || !value.bytes().all(|byte| byte.is_ascii_digit())
        || (value.len() > 1 && value.starts_with('0'))
    {
        return Err("product_plan_decimal");
    }
    value.parse().map_err(|_| "product_plan_decimal")
}
