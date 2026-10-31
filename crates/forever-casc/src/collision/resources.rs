// SPDX-License-Identifier: GPL-3.0-or-later
// Authored 2026-10-12: explicit cumulative disk-world resource capability.
//! Original defaults and per-file/parser/memory ceilings are unchanged. This
//! versioned opt-in raises only declared cumulative/world capacities. The actual
//! six spool owners carry the capability; mixing policies is rejected before IO.
//! Every configured value may only reduce its profile maximum. Logical memory
//! and CPU-work accounting retain their existing scope, not RSS/all CPU work.
use super::{assemble::staged, gameobjects, inventory_spool, spool};
#[cfg(test)]
mod tests;
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Profile {
    #[default]
    BoundedV1,
    DiskWorldV1,
    RequestHeavyV1,
    DependencyHeavyV1,
    SourceHeavyV1,
    ReadHeavyV1,
    ModelHeavyV1,
    SourceExpandedV1,
    ModelOutputExpandedV1,
    ModelBytesExpandedV1,
    SourceExpandedV2,
    PlacementExpandedV1,
    AssemblyExpandedV1,
    GameObjectExpandedV1,
    GameObjectMissingRootExpandedV1,
    GameObjectRootAndEncryptedUnavailableExpandedV1,
}
fn wide(bytes: u64) -> usize {
    if usize::BITS >= 64 {
        bytes as usize
    } else {
        usize::MAX
    }
}
impl Profile {
    pub const fn id(self) -> &'static str {
        match self {
            Self::BoundedV1 => "linux-collision-bounded-resources-v1",
            Self::DiskWorldV1 => "linux-collision-disk-world-resources-v1",
            Self::RequestHeavyV1 => "linux-collision-request-heavy-resources-v1",
            Self::DependencyHeavyV1 => "linux-collision-dependency-heavy-resources-v1",
            Self::SourceHeavyV1 => "linux-collision-source-heavy-resources-v1",
            Self::ReadHeavyV1 => "linux-collision-read-heavy-resources-v1",
            Self::ModelHeavyV1 => "linux-collision-model-heavy-resources-v1",
            Self::SourceExpandedV1 => "linux-collision-source-expanded-resources-v1",
            Self::ModelOutputExpandedV1 => "linux-collision-model-output-expanded-resources-v1",
            Self::ModelBytesExpandedV1 => "linux-collision-model-bytes-expanded-resources-v1",
            Self::SourceExpandedV2 => "linux-collision-source-expanded-resources-v2",
            Self::PlacementExpandedV1 => "linux-collision-placement-expanded-resources-v1",
            Self::AssemblyExpandedV1 => "linux-collision-assembly-expanded-resources-v1",
            Self::GameObjectExpandedV1 => "linux-collision-gameobject-expanded-resources-v1",
            Self::GameObjectMissingRootExpandedV1 => {
                "linux-collision-gameobject-missing-root-expanded-resources-v1"
            }
            Self::GameObjectRootAndEncryptedUnavailableExpandedV1 => {
                "linux-collision-gameobject-root-encrypted-unavailable-expanded-resources-v1"
            }
        }
    }
    pub(crate) const fn model_heavy(self) -> bool {
        matches!(
            self,
            Self::ModelHeavyV1
                | Self::SourceExpandedV1
                | Self::ModelOutputExpandedV1
                | Self::ModelBytesExpandedV1
                | Self::SourceExpandedV2
                | Self::PlacementExpandedV1
                | Self::AssemblyExpandedV1
                | Self::GameObjectExpandedV1
                | Self::GameObjectMissingRootExpandedV1
                | Self::GameObjectRootAndEncryptedUnavailableExpandedV1
        )
    }
    pub(crate) const fn supported(self) -> bool {
        match self {
            Self::BoundedV1 => true,
            Self::DiskWorldV1 => usize::BITS >= 64,
            Self::RequestHeavyV1
            | Self::DependencyHeavyV1
            | Self::SourceHeavyV1
            | Self::ReadHeavyV1
            | Self::ModelHeavyV1
            | Self::SourceExpandedV1
            | Self::ModelOutputExpandedV1
            | Self::ModelBytesExpandedV1
            | Self::SourceExpandedV2
            | Self::PlacementExpandedV1
            | Self::AssemblyExpandedV1
            | Self::GameObjectExpandedV1
            | Self::GameObjectMissingRootExpandedV1
            | Self::GameObjectRootAndEncryptedUnavailableExpandedV1 => cfg!(target_arch = "x86_64"),
        }
    }
    pub fn collector_limits(self) -> super::Limits {
        let mut d = super::Limits::default();
        if self == Self::DiskWorldV1 {
            d.max_reads = 1_000_000;
            d.max_models = 65_536;
            d.max_requests = 16_000_000;
            d.max_dependency_requests = 16_000_000;
            d.max_source_bytes = wide(256 * 1024 * 1024 * 1024);
            d.max_raw_bytes = wide(64 * 1024 * 1024 * 1024);
        }
        if matches!(
            self,
            Self::RequestHeavyV1
                | Self::DependencyHeavyV1
                | Self::SourceHeavyV1
                | Self::ReadHeavyV1
                | Self::ModelHeavyV1
                | Self::SourceExpandedV1
                | Self::ModelOutputExpandedV1
                | Self::ModelBytesExpandedV1
                | Self::SourceExpandedV2
                | Self::PlacementExpandedV1
                | Self::AssemblyExpandedV1
                | Self::GameObjectExpandedV1
                | Self::GameObjectMissingRootExpandedV1
                | Self::GameObjectRootAndEncryptedUnavailableExpandedV1
        ) {
            d.max_requests = 16_000_000;
        }
        if matches!(
            self,
            Self::DependencyHeavyV1
                | Self::SourceHeavyV1
                | Self::ReadHeavyV1
                | Self::ModelHeavyV1
                | Self::SourceExpandedV1
                | Self::ModelOutputExpandedV1
                | Self::ModelBytesExpandedV1
                | Self::SourceExpandedV2
                | Self::PlacementExpandedV1
                | Self::AssemblyExpandedV1
                | Self::GameObjectExpandedV1
                | Self::GameObjectMissingRootExpandedV1
                | Self::GameObjectRootAndEncryptedUnavailableExpandedV1
        ) {
            d.max_dependency_requests = 16_000_000;
        }
        if matches!(
            self,
            Self::SourceHeavyV1
                | Self::ReadHeavyV1
                | Self::ModelHeavyV1
                | Self::SourceExpandedV1
                | Self::ModelOutputExpandedV1
                | Self::ModelBytesExpandedV1
                | Self::SourceExpandedV2
                | Self::PlacementExpandedV1
                | Self::AssemblyExpandedV1
                | Self::GameObjectExpandedV1
                | Self::GameObjectMissingRootExpandedV1
                | Self::GameObjectRootAndEncryptedUnavailableExpandedV1
        ) {
            d.max_source_bytes = 512 * 1024 * 1024;
        }
        if matches!(
            self,
            Self::SourceExpandedV1 | Self::ModelOutputExpandedV1 | Self::ModelBytesExpandedV1
        ) {
            d.max_source_bytes = 1024 * 1024 * 1024;
        }
        if matches!(
            self,
            Self::SourceExpandedV2
                | Self::PlacementExpandedV1
                | Self::AssemblyExpandedV1
                | Self::GameObjectExpandedV1
                | Self::GameObjectMissingRootExpandedV1
                | Self::GameObjectRootAndEncryptedUnavailableExpandedV1
        ) {
            d.max_source_bytes = 2 * 1024 * 1024 * 1024;
        }
        if self == Self::ReadHeavyV1 || self.model_heavy() {
            d.max_reads = 1_000_000;
        }
        if self.model_heavy() {
            d.max_models = 65_536;
        }
        d
    }
    pub fn spool_limits(self) -> spool::Limits {
        let mut d = spool::Limits::default();
        if self == Self::DiskWorldV1 {
            d.max_records = 262_144;
            d.max_disk_bytes = wide(128 * 1024 * 1024 * 1024);
            d.max_descriptor_bytes = 64 * 1024 * 1024;
            d.max_reloads = 16_000_000;
            d.max_reload_bytes = wide(4 * 1024 * 1024 * 1024 * 1024);
        }
        if self.model_heavy() {
            d.max_records = self.collector_limits().max_models;
        }
        d
    }
    pub fn list_profile(self) -> forever_vmap::gameobject_models::streaming::BudgetProfile {
        use forever_vmap::gameobject_models::streaming::BudgetProfile;
        match self {
            Self::BoundedV1
            | Self::RequestHeavyV1
            | Self::DependencyHeavyV1
            | Self::SourceHeavyV1
            | Self::ReadHeavyV1
            | Self::ModelHeavyV1
            | Self::SourceExpandedV1
            | Self::ModelOutputExpandedV1
            | Self::ModelBytesExpandedV1
            | Self::SourceExpandedV2
            | Self::PlacementExpandedV1
            | Self::AssemblyExpandedV1
            | Self::GameObjectExpandedV1
            | Self::GameObjectMissingRootExpandedV1
            | Self::GameObjectRootAndEncryptedUnavailableExpandedV1 => BudgetProfile::BoundedV1,
            Self::DiskWorldV1 => BudgetProfile::RepeatedRowsV1,
        }
    }
    pub fn startup_limits(self) -> gameobjects::Limits {
        let mut d = gameobjects::Limits::default();
        if self == Self::DiskWorldV1 {
            d.collision = self.collector_limits();
            d.max_candidate_requests = gameobjects::MAX_DIRECT_ROWS;
            d.lists = self.list_profile().limits();
        }
        if matches!(
            self,
            Self::RequestHeavyV1
                | Self::DependencyHeavyV1
                | Self::SourceHeavyV1
                | Self::ReadHeavyV1
                | Self::ModelHeavyV1
                | Self::SourceExpandedV1
                | Self::ModelOutputExpandedV1
                | Self::ModelBytesExpandedV1
                | Self::SourceExpandedV2
                | Self::PlacementExpandedV1
                | Self::AssemblyExpandedV1
                | Self::GameObjectExpandedV1
                | Self::GameObjectMissingRootExpandedV1
                | Self::GameObjectRootAndEncryptedUnavailableExpandedV1
        ) {
            d.collision.max_requests = 16_000_000;
        }
        if matches!(
            self,
            Self::DependencyHeavyV1
                | Self::SourceHeavyV1
                | Self::ReadHeavyV1
                | Self::ModelHeavyV1
                | Self::SourceExpandedV1
                | Self::ModelOutputExpandedV1
                | Self::ModelBytesExpandedV1
                | Self::SourceExpandedV2
                | Self::PlacementExpandedV1
                | Self::AssemblyExpandedV1
                | Self::GameObjectExpandedV1
                | Self::GameObjectMissingRootExpandedV1
                | Self::GameObjectRootAndEncryptedUnavailableExpandedV1
        ) {
            d.collision.max_dependency_requests = 16_000_000;
        }
        if matches!(
            self,
            Self::SourceHeavyV1
                | Self::ReadHeavyV1
                | Self::ModelHeavyV1
                | Self::SourceExpandedV1
                | Self::ModelOutputExpandedV1
                | Self::ModelBytesExpandedV1
                | Self::SourceExpandedV2
                | Self::PlacementExpandedV1
                | Self::AssemblyExpandedV1
                | Self::GameObjectExpandedV1
                | Self::GameObjectMissingRootExpandedV1
                | Self::GameObjectRootAndEncryptedUnavailableExpandedV1
        ) {
            d.collision.max_source_bytes = 512 * 1024 * 1024;
        }
        if matches!(
            self,
            Self::SourceExpandedV1 | Self::ModelOutputExpandedV1 | Self::ModelBytesExpandedV1
        ) {
            d.collision.max_source_bytes = 1024 * 1024 * 1024;
        }
        if matches!(
            self,
            Self::SourceExpandedV2
                | Self::PlacementExpandedV1
                | Self::AssemblyExpandedV1
                | Self::GameObjectExpandedV1
                | Self::GameObjectMissingRootExpandedV1
                | Self::GameObjectRootAndEncryptedUnavailableExpandedV1
        ) {
            d.collision.max_source_bytes = 2 * 1024 * 1024 * 1024;
        }
        if self == Self::ReadHeavyV1 || self.model_heavy() {
            d.collision.max_reads = 1_000_000;
        }
        if self.model_heavy() {
            d.collision.max_models = 65_536;
        }
        d
    }
    pub fn inventory_limits(self) -> inventory_spool::Limits {
        let mut d = inventory_spool::Limits::default();
        if self == Self::DiskWorldV1 {
            d.max_entries = self.spool_limits().max_records;
            d.max_heap_bytes = 256 * 1024 * 1024;
            // Two 64 MiB descriptor ceilings plus guards leave less than
            // 128 MiB inside this inventory. Keep the cache admission allowance
            // below that remainder; this only reduces the existing parser cap.
            d.cache.max_memory_bytes = 64 * 1024 * 1024;
        }
        d
    }
    pub fn stage_limits(self) -> staged::Limits {
        let mut d = staged::Limits::default();
        if self == Self::DiskWorldV1 {
            d.max_callbacks = self.spool_limits().max_records;
            d.world.world.run.cache = self.inventory_limits().cache;
            d.max_records = 8_000_000;
            d.world.world.run.placements.max_models = self.collector_limits().max_models;
        }
        if self.model_heavy() {
            d.world.world.run.placements.max_models = self.collector_limits().max_models;
        }
        d
    }
    pub fn model_limits(self) -> staged::models::Limits {
        let mut d = staged::models::Limits::default();
        if self == Self::DiskWorldV1 {
            d.max_models = self.collector_limits().max_models;
            d.max_output_bytes = wide(64 * 1024 * 1024 * 1024);
            d.max_vertices = 256_000_000;
            d.max_triangles = 256_000_000;
            d.max_raw_visit_bytes = 256 * 1024 * 1024 * 1024;
            d.max_bih_work = 8_000_000_000;
            d.max_validation_bytes = 4 * 1024 * 1024 * 1024 * 1024;
        }
        if self.model_heavy() {
            d.max_models = self.collector_limits().max_models;
        }
        d
    }
    pub fn package_limits(self) -> staged::package::Limits {
        let mut d = staged::package::Limits::default();
        if self == Self::DiskWorldV1 {
            let m = self.model_limits();
            d.max_models = m.max_models;
            d.max_output_bytes = m.max_output_bytes;
            d.max_vertices = m.max_vertices;
            d.max_triangles = m.max_triangles;
            d.max_raw_visit_bytes = m.max_raw_visit_bytes;
            d.max_bih_work = m.max_bih_work;
            d.max_validation_bytes = m.max_validation_bytes;
            d.max_files = 1_000_000;
            d.max_records = self.stage_limits().max_records;
            d.max_associations = 256_000_000;
            d.max_ancestor_steps = 256_000_000;
        }
        if self.model_heavy() {
            d.max_models = self.model_limits().max_models;
        }
        d
    }
}
