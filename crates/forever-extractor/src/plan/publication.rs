// SPDX-License-Identifier: GPL-3.0-or-later
//! Explicit Linux non-geometry component publication, separate from DB2 metadata.
use super::{CollisionStorage, Component, Plan, SchemaCatalog, Selection};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum PublicationProfile {
    #[default]
    Legacy,
    Build70338Db2GtCamerasV1,
    Build70374Db2OnlyV1,
    Build70374Db2GtCamerasV1,
}
impl PublicationProfile {
    pub(super) fn validate(self, plan: &Plan) -> Result<(), &'static str> {
        if self == Self::Legacy {
            return Ok(());
        }
        let (build, catalog, components) = match self {
            Self::Legacy => unreachable!("legacy returned above"),
            Self::Build70338Db2GtCamerasV1 => (
                70338,
                SchemaCatalog::WowDbDefs70338V1,
                Component::Db2.bit() | Component::GameTables.bit() | Component::Cameras.bit(),
            ),
            Self::Build70374Db2GtCamerasV1 => (
                70374,
                SchemaCatalog::WowDbDefs70374V1,
                Component::Db2.bit() | Component::GameTables.bit() | Component::Cameras.bit(),
            ),
            Self::Build70374Db2OnlyV1 => {
                (70374, SchemaCatalog::WowDbDefs70374V1, Component::Db2.bit())
            }
        };
        if plan.build != build {
            return Err("product_plan_build");
        }
        if plan.schema_catalog != catalog {
            return Err("product_plan_schema_catalog");
        }
        if plan.components != components
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
