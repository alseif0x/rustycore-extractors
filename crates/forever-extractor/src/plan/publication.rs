// SPDX-License-Identifier: GPL-3.0-or-later
//! Explicit Linux non-geometry component publication, separate from DB2 metadata.
use super::{CollisionStorage, Component, Plan, SchemaCatalog, Selection};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum PublicationProfile {
    #[default]
    Legacy,
    Build70338Db2GtCamerasV1,
}
impl PublicationProfile {
    pub(super) fn validate(self, plan: &Plan) -> Result<(), &'static str> {
        if self == Self::Legacy {
            return Ok(());
        }
        if plan.build != 70338 {
            return Err("product_plan_build");
        }
        if plan.schema_catalog != SchemaCatalog::WowDbDefs70338V1 {
            return Err("product_plan_schema_catalog");
        }
        if plan.components
            != Component::Db2.bit() | Component::GameTables.bit() | Component::Cameras.bit()
            || !matches!(plan.collision_storage, CollisionStorage::Resident)
            || plan.map_domain.is_some()
            || plan.offmesh.is_some()
        {
            return Err("product_plan_component_scope");
        }
        if plan.terrain != Selection::None || plan.navigation != Selection::None {
            return Err("product_plan_dependencies");
        }
        Ok(())
    }
}
