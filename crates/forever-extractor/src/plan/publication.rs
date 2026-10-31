// SPDX-License-Identifier: GPL-3.0-or-later
// Modified by the RustyCore Extractors project, 2026-10-19.
//! Explicit Linux component publication, separate from DB2 metadata.
use super::{CollisionStorage, Component, Plan, SchemaCatalog, Selection};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum PublicationProfile {
    #[default]
    Legacy,
    Build70338Db2GtCamerasV1,
    Build70374Db2OnlyV1,
    Build70374Db2GtCamerasV1,
    Build70374AvailableTerrainV1,
    Build70374CompatibleTerrainV1,
    Build70450CompatibleTerrainV1,
    Build70425CompatibleCollisionV1,
    Build70450CompatibleCollisionV1,
    Build70450CollisionProvidedKeysV1,
    Build70450Db2OnlyV1,
    Build70450Db2PartialV1,
    Build70450Db2ProvidedKeysV1,
    Build70450Db2GtCamerasV1,
    Build70450NavigationV1,
    Build70450SelectedGeometryV1,
    Build70450SelectedNavigationV1,
    Build70450AvailableNavigationV1,
    Build70450AvailableNavigationV2,
    Build70450AvailableNavigationV3,
    Build70450AvailableNavigationV4,
    Build70450AvailableNavigationV5,
    Build70450AvailableNavigationV6,
    Build70450AuthenticatedTerrainV1,
    Build70450AuthenticatedTerrainPartialMapV1,
    Build70450PartialCatalogTerrainV1,
    Build70450AuthenticatedSelectedGeometryV1,
    Build70450AuthenticatedAvailableGeometryV1,
    Build70450AvailableRootAbsentGeometryV1,
    Build70450AvailableModelRootAbsentGeometryV1,
    Build70450AvailableModelRootAbsentRequestCapacityGeometryV1,
    Build70450AvailableVacatedWmoGeometryV1,
    Build70450AvailableEmptyWmoGeometryV1,
    Build70450AvailableUnselectedWmoGeometryV1,
    Build70450MaterialFilteredWmoGeometryV1,
    Build70450DependencyHeavyMaterialFilteredWmoGeometryV1,
    Build70450SourceHeavyMaterialFilteredWmoGeometryV1,
    Build70450ReadHeavyMaterialFilteredWmoGeometryV1,
    Build70450ModelHeavyMaterialFilteredWmoGeometryV1,
    Build70450SourceExpandedMaterialFilteredWmoGeometryV1,
    Build70450ModelOutputExpandedMaterialFilteredWmoGeometryV1,
    Build70450ModelBytesExpandedMaterialFilteredWmoGeometryV1,
    Build70450SourceExpandedV2MaterialFilteredWmoGeometryV1,
    Build70450PlacementExpandedMaterialFilteredWmoGeometryV1,
    Build70450AssemblyExpandedMaterialFilteredWmoGeometryV1,
    Build70450GameObjectExpandedMaterialFilteredWmoGeometryV1,
    Build70450GameObjectMissingRootMaterialFilteredWmoGeometryV1,
    Build70450GameObjectRootAndEncryptedUnavailableMaterialFilteredWmoGeometryV1,
    Build70450LiquidObjectCompleteSourceV1,
    Build70450LiquidTerrainVmapV1,
    Build70450LiquidTerrainVmapCombinedV1,
    Build70450Db2TerrainLiquidVmapV1,
}
impl PublicationProfile {
    pub(super) fn validate(self, plan: &Plan) -> Result<(), &'static str> {
        if self == Self::Legacy {
            return Ok(());
        }
        if self == Self::Build70425CompatibleCollisionV1 {
            if plan.build != 70425 {
                return Err("product_plan_build");
            }
            if plan.schema_catalog != SchemaCatalog::WowDbDefs70425V1 {
                return Err("product_plan_schema_catalog");
            }
            if plan.components != Component::Collision.bit()
                || !matches!(plan.collision_storage, CollisionStorage::LeasedDiskV1(_))
                || plan.decryption != super::Decryption::Disabled
                || !plan.db2.is_empty()
                || plan.terrain != Selection::None
                || plan.navigation != Selection::None
                || plan.offmesh.is_some()
                || !plan.pack
                || plan.big_base_unit
                || plan.skip_liquids
                || plan.maximum_slope != 55
                || plan.non_steep_slope != 55
            {
                return Err("product_plan_component_scope");
            }
            return Ok(());
        }
        if self == Self::Build70450CompatibleCollisionV1 {
            if plan.build != 70450 {
                return Err("product_plan_build");
            }
            if plan.schema_catalog != SchemaCatalog::WowDbDefs70450V1 {
                return Err("product_plan_schema_catalog");
            }
            if plan.components != Component::Collision.bit()
                || !matches!(plan.collision_storage, CollisionStorage::LeasedDiskV1(_))
                || plan.decryption != super::Decryption::Disabled
                || !plan.db2.is_empty()
                || plan.terrain != Selection::None
                || plan.navigation != Selection::None
                || plan.offmesh.is_some()
                || !plan.pack
                || plan.big_base_unit
                || plan.skip_liquids
                || plan.maximum_slope != 55
                || plan.non_steep_slope != 55
            {
                return Err("product_plan_component_scope");
            }
            return Ok(());
        }
        if self == Self::Build70450CollisionProvidedKeysV1 {
            if plan.build != 70450 {
                return Err("product_plan_build");
            }
            if plan.schema_catalog != SchemaCatalog::WowDbDefs70450V1 {
                return Err("product_plan_schema_catalog");
            }
            if plan.components != Component::Collision.bit()
                || !matches!(plan.collision_storage, CollisionStorage::LeasedDiskV1(_))
                || plan.decryption != super::Decryption::ProvidedSalsa20V1
                || !plan.db2.is_empty()
                || plan.terrain != Selection::None
                || plan.navigation != Selection::None
                || plan.offmesh.is_some()
                || !plan.pack
                || plan.big_base_unit
                || plan.skip_liquids
                || plan.maximum_slope != 55
                || plan.non_steep_slope != 55
            {
                return Err("product_plan_component_scope");
            }
            return Ok(());
        }
        if self == Self::Build70450LiquidObjectCompleteSourceV1 {
            if plan.build != 70450 {
                return Err("product_plan_build");
            }
            if plan.schema_catalog != SchemaCatalog::WowDbDefs70450V1 {
                return Err("product_plan_schema_catalog");
            }
            if plan.components != (Component::Db2.bit() | Component::Collision.bit())
                || !matches!(plan.collision_storage, CollisionStorage::LeasedDiskV1(_))
                || plan.decryption != super::Decryption::Disabled
                || plan.map_domain.is_some()
                || plan.terrain != Selection::None
                || plan.navigation != Selection::None
                || plan.offmesh.is_some()
                || !plan.pack
                || plan.big_base_unit
                || plan.skip_liquids
                || plan.maximum_slope != 55
                || plan.non_steep_slope != 55
            {
                return Err("product_plan_component_scope");
            }
            if plan.db2.len() != 3
                || plan.db2[0].name != "GameObjectDisplayInfo"
                || plan.db2[1].name != "Map"
                || plan.db2[2].name != "LiquidObject"
            {
                return Err("product_plan_dependencies");
            }
            let CollisionStorage::LeasedDiskV1(budget) = plan.collision_storage else {
                return Err("product_plan_component_scope");
            };
            if budget.raw_bytes > 2 * 1024 * 1024 * 1024
                || budget.output_bytes > 2 * 1024 * 1024 * 1024
                || budget.wdt_bytes > 256 * 1024 * 1024
                || budget.cache_bytes > 256 * 1024 * 1024
                || budget.placement_bytes > 256 * 1024 * 1024
                || budget.map_bytes > 256 * 1024 * 1024
                || budget.vmo_bytes > 512 * 1024 * 1024
            {
                return Err("product_plan_disk_budget");
            }
            return Ok(());
        }
        if self == Self::Build70450Db2TerrainLiquidVmapV1 {
            if plan.build != 70450 {
                return Err("product_plan_build");
            }
            if plan.schema_catalog != SchemaCatalog::WowDbDefs70450V1 {
                return Err("product_plan_schema_catalog");
            }
            let components =
                Component::Db2.bit() | Component::Terrain.bit() | Component::Collision.bit();
            if plan.components != components
                || !matches!(plan.collision_storage, CollisionStorage::LeasedDiskV1(_))
                || plan.decryption != super::Decryption::Disabled
                || plan.map_domain.is_some()
                || plan.terrain != Selection::All
                || plan.navigation != Selection::None
                || plan.offmesh.is_some()
                || !plan.pack
                || plan.big_base_unit
                || plan.skip_liquids
                || plan.maximum_slope != 55
                || plan.non_steep_slope != 55
            {
                return Err("product_plan_component_scope");
            }
            if plan.db2.len() != 5
                || plan.db2[0].name != "GameObjectDisplayInfo"
                || plan.db2[1].name != "Map"
                || plan.db2[2].name != "LiquidObject"
                || plan.db2[3].name != "LiquidMaterial"
                || plan.db2[4].name != "LiquidType"
                || plan.db2[0].id != 1_266_277
                || plan.db2[1].id != 1_349_477
                || plan.db2[2].id != 1_308_058
                || plan.db2[3].id != 1_132_538
                || plan.db2[4].id != 1_371_380
            {
                return Err("product_plan_dependencies");
            }
            let CollisionStorage::LeasedDiskV1(budget) = plan.collision_storage else {
                return Err("product_plan_component_scope");
            };
            if budget.raw_bytes > 2 * 1024 * 1024 * 1024
                || budget.output_bytes > 2 * 1024 * 1024 * 1024
                || budget.wdt_bytes > 256 * 1024 * 1024
                || budget.cache_bytes > 256 * 1024 * 1024
                || budget.placement_bytes > 256 * 1024 * 1024
                || budget.map_bytes > 256 * 1024 * 1024
                || budget.vmo_bytes > 512 * 1024 * 1024
            {
                return Err("product_plan_disk_budget");
            }
            return Ok(());
        }
        if self == Self::Build70450LiquidTerrainVmapCombinedV1 {
            if plan.build != 70450 {
                return Err("product_plan_build");
            }
            if plan.schema_catalog != SchemaCatalog::WowDbDefs70450V1 {
                return Err("product_plan_schema_catalog");
            }
            if plan.components
                != (Component::Db2.bit() | Component::Terrain.bit() | Component::Collision.bit())
                || !matches!(plan.collision_storage, CollisionStorage::LeasedDiskV1(_))
                || plan.decryption != super::Decryption::Disabled
                || plan.map_domain.is_some()
                || plan.terrain != Selection::All
                || plan.navigation != Selection::None
                || plan.offmesh.is_some()
                || !plan.pack
                || plan.big_base_unit
                || plan.skip_liquids
                || plan.maximum_slope != 55
                || plan.non_steep_slope != 55
            {
                return Err("product_plan_component_scope");
            }
            if plan.db2.len() != 5
                || plan.db2[0].name != "GameObjectDisplayInfo"
                || plan.db2[1].name != "Map"
                || plan.db2[2].name != "LiquidObject"
                || plan.db2[3].name != "LiquidMaterial"
                || plan.db2[4].name != "LiquidType"
            {
                return Err("product_plan_dependencies");
            }
            let CollisionStorage::LeasedDiskV1(budget) = plan.collision_storage else {
                return Err("product_plan_component_scope");
            };
            if budget.raw_bytes > 2 * 1024 * 1024 * 1024
                || budget.output_bytes > 2 * 1024 * 1024 * 1024
                || budget.wdt_bytes > 256 * 1024 * 1024
                || budget.cache_bytes > 256 * 1024 * 1024
                || budget.placement_bytes > 256 * 1024 * 1024
                || budget.map_bytes > 256 * 1024 * 1024
                || budget.vmo_bytes > 512 * 1024 * 1024
            {
                return Err("product_plan_disk_budget");
            }
            return Ok(());
        }
        if self == Self::Build70450LiquidTerrainVmapV1 {
            if plan.build != 70450 {
                return Err("product_plan_build");
            }
            if plan.schema_catalog != SchemaCatalog::WowDbDefs70450V1 {
                return Err("product_plan_schema_catalog");
            }
            if plan.components
                != (Component::Db2.bit() | Component::Terrain.bit() | Component::Collision.bit())
                || !matches!(plan.collision_storage, CollisionStorage::LeasedDiskV1(_))
                || plan.decryption != super::Decryption::Disabled
                || plan.map_domain.is_some()
                || plan.terrain != Selection::All
                || plan.navigation != Selection::None
                || plan.offmesh.is_some()
                || !plan.pack
                || plan.big_base_unit
                || plan.skip_liquids
                || plan.maximum_slope != 55
                || plan.non_steep_slope != 55
            {
                return Err("product_plan_component_scope");
            }
            if plan.db2.len() != 5
                || plan.db2[0].name != "GameObjectDisplayInfo"
                || plan.db2[1].name != "Map"
                || plan.db2[2].name != "LiquidObject"
                || plan.db2[3].name != "LiquidMaterial"
                || plan.db2[4].name != "LiquidType"
            {
                return Err("product_plan_dependencies");
            }
            let CollisionStorage::LeasedDiskV1(budget) = plan.collision_storage else {
                return Err("product_plan_component_scope");
            };
            if budget.raw_bytes > 2 * 1024 * 1024 * 1024
                || budget.output_bytes > 2 * 1024 * 1024 * 1024
                || budget.wdt_bytes > 256 * 1024 * 1024
                || budget.cache_bytes > 256 * 1024 * 1024
                || budget.placement_bytes > 256 * 1024 * 1024
                || budget.map_bytes > 256 * 1024 * 1024
                || budget.vmo_bytes > 512 * 1024 * 1024
            {
                return Err("product_plan_disk_budget");
            }
            return Ok(());
        }
        if self == Self::Build70450Db2PartialV1 {
            if plan.build != 70450 {
                return Err("product_plan_build");
            }
            if plan.schema_catalog != SchemaCatalog::WowDbDefs70450V1 {
                return Err("product_plan_schema_catalog");
            }
            if plan.components != Component::Db2.bit()
                || !matches!(plan.collision_storage, CollisionStorage::Resident)
                || plan.decryption != super::Decryption::Disabled
                || plan.map_domain.is_some()
                || plan.terrain != Selection::None
                || plan.navigation != Selection::None
                || plan.offmesh.is_some()
                || !plan.pack
                || plan.big_base_unit
                || plan.skip_liquids
                || plan.maximum_slope != 55
                || plan.non_steep_slope != 55
            {
                return Err("product_plan_component_scope");
            }
            return Ok(());
        }
        if self == Self::Build70450CompatibleTerrainV1 {
            if plan.build != 70450 {
                return Err("product_plan_build");
            }
            if plan.schema_catalog != SchemaCatalog::WowDbDefs70450V1 {
                return Err("product_plan_schema_catalog");
            }
            if plan.components != Component::Terrain.bit()
                || !matches!(plan.collision_storage, CollisionStorage::Resident)
                || plan.decryption != super::Decryption::Disabled
                || plan.map_domain.is_some()
                || plan.offmesh.is_some()
            {
                return Err("product_plan_component_scope");
            }
            if !plan.db2.is_empty() || plan.navigation != Selection::None {
                return Err("product_plan_dependencies");
            }
            if !matches!(plan.terrain, Selection::All | Selection::Map(_)) {
                return Err("product_terrain_selection");
            }
            return Ok(());
        }
        if self == Self::Build70450NavigationV1 {
            if plan.build != 70450 {
                return Err("product_plan_build");
            }
            if plan.schema_catalog != SchemaCatalog::WowDbDefs70450V1 {
                return Err("product_plan_schema_catalog");
            }
            let components = Component::Db2.bit()
                | Component::Terrain.bit()
                | Component::Collision.bit()
                | Component::Navigation.bit();
            if plan.components != components
                || !matches!(plan.collision_storage, CollisionStorage::LeasedDiskV1(_))
                || plan.decryption != super::Decryption::Disabled
                || plan.map_domain.is_none()
                || !plan.db2.iter().any(|entry| entry.name == "Map")
                || !plan.db2.iter().any(|entry| entry.name == "LiquidType")
            {
                return Err("product_plan_component_scope");
            }
            let map_id = plan.map_domain.ok_or("product_plan_map_domain")?;
            if plan.terrain != Selection::Map(map_id) || plan.navigation != Selection::Map(map_id) {
                return Err("product_plan_map_domain_selection");
            }
            return Ok(());
        }
        if self == Self::Build70450SelectedGeometryV1 {
            if plan.build != 70450 {
                return Err("product_plan_build");
            }
            if plan.schema_catalog != SchemaCatalog::WowDbDefs70450V1 {
                return Err("product_plan_schema_catalog");
            }
            if plan.components != Component::Collision.bit()
                || !matches!(plan.collision_storage, CollisionStorage::LeasedDiskV1(_))
                || plan.decryption != super::Decryption::Disabled
                || !plan.db2.is_empty()
                || plan.terrain != Selection::None
                || plan.navigation != Selection::None
                || plan.offmesh.is_some()
                || !plan.pack
                || plan.big_base_unit
                || plan.skip_liquids
                || plan.maximum_slope != 55
                || plan.non_steep_slope != 55
            {
                return Err("product_plan_component_scope");
            }
            return Ok(());
        }
        if self == Self::Build70450AuthenticatedSelectedGeometryV1 {
            if plan.build != 70450 {
                return Err("product_plan_build");
            }
            if plan.schema_catalog != SchemaCatalog::WowDbDefs70450V1 {
                return Err("product_plan_schema_catalog");
            }
            if plan.components != (Component::Db2.bit() | Component::Collision.bit())
                || !matches!(plan.collision_storage, CollisionStorage::LeasedDiskV1(_))
                || plan.decryption != super::Decryption::Disabled
                || plan.map_domain.is_none()
                || plan.terrain != Selection::None
                || plan.navigation != Selection::None
                || plan.offmesh.is_some()
                || !plan.pack
                || plan.big_base_unit
                || plan.skip_liquids
                || plan.maximum_slope != 55
                || plan.non_steep_slope != 55
            {
                return Err("product_plan_component_scope");
            }
            if plan.db2.len() != 1 || plan.db2[0].name != "Map" {
                return Err("product_plan_dependencies");
            }
            return Ok(());
        }
        if matches!(
            self,
            Self::Build70450GameObjectExpandedMaterialFilteredWmoGeometryV1
                | Self::Build70450GameObjectMissingRootMaterialFilteredWmoGeometryV1
                | Self::Build70450GameObjectRootAndEncryptedUnavailableMaterialFilteredWmoGeometryV1
        ) {
            if plan.build != 70450 {
                return Err("product_plan_build");
            }
            if plan.schema_catalog != SchemaCatalog::WowDbDefs70450V1 {
                return Err("product_plan_schema_catalog");
            }
            if plan.components != (Component::Db2.bit() | Component::Collision.bit())
                || !matches!(plan.collision_storage, CollisionStorage::LeasedDiskV1(_))
                || plan.decryption != super::Decryption::Disabled
                || plan.map_domain.is_some()
                || plan.terrain != Selection::None
                || plan.navigation != Selection::None
                || plan.offmesh.is_some()
                || !plan.pack
                || plan.big_base_unit
                || plan.skip_liquids
                || plan.maximum_slope != 55
                || plan.non_steep_slope != 55
            {
                return Err("product_plan_component_scope");
            }
            if plan.db2.len() != 2
                || plan.db2[0].name != "GameObjectDisplayInfo"
                || plan.db2[1].name != "Map"
            {
                return Err("product_plan_dependencies");
            }
            if matches!(
                self,
                Self::Build70450GameObjectMissingRootMaterialFilteredWmoGeometryV1
                    | Self::Build70450GameObjectRootAndEncryptedUnavailableMaterialFilteredWmoGeometryV1
            ) {
                let CollisionStorage::LeasedDiskV1(budget) = plan.collision_storage else {
                    return Err("product_plan_component_scope");
                };
                if budget.raw_bytes > 2 * 1024 * 1024 * 1024
                    || budget.output_bytes > 2 * 1024 * 1024 * 1024
                    || budget.wdt_bytes > 256 * 1024 * 1024
                    || budget.cache_bytes > 256 * 1024 * 1024
                    || budget.placement_bytes > 256 * 1024 * 1024
                    || budget.map_bytes > 256 * 1024 * 1024
                    || budget.vmo_bytes > 512 * 1024 * 1024
                {
                    return Err("product_plan_disk_budget");
                }
            }
            return Ok(());
        }
        if matches!(
            self,
            Self::Build70450ModelOutputExpandedMaterialFilteredWmoGeometryV1
                | Self::Build70450ModelBytesExpandedMaterialFilteredWmoGeometryV1
                | Self::Build70450SourceExpandedV2MaterialFilteredWmoGeometryV1
                | Self::Build70450PlacementExpandedMaterialFilteredWmoGeometryV1
                | Self::Build70450AssemblyExpandedMaterialFilteredWmoGeometryV1
        ) {
            if plan.build != 70450 {
                return Err("product_plan_build");
            }
            if plan.schema_catalog != SchemaCatalog::WowDbDefs70450V1 {
                return Err("product_plan_schema_catalog");
            }
            if plan.components != (Component::Db2.bit() | Component::Collision.bit())
                || !matches!(plan.collision_storage, CollisionStorage::LeasedDiskV1(_))
                || plan.decryption != super::Decryption::Disabled
                || plan.map_domain.is_some()
                || plan.terrain != Selection::None
                || plan.navigation != Selection::None
                || plan.offmesh.is_some()
                || !plan.pack
                || plan.big_base_unit
                || plan.skip_liquids
                || plan.maximum_slope != 55
                || plan.non_steep_slope != 55
            {
                return Err("product_plan_component_scope");
            }
            if plan.db2.len() != 1 || plan.db2[0].name != "Map" {
                return Err("product_plan_dependencies");
            }
            return Ok(());
        }
        if matches!(
            self,
            Self::Build70450AuthenticatedAvailableGeometryV1
                | Self::Build70450AvailableRootAbsentGeometryV1
                | Self::Build70450AvailableModelRootAbsentGeometryV1
                | Self::Build70450AvailableModelRootAbsentRequestCapacityGeometryV1
                | Self::Build70450AvailableVacatedWmoGeometryV1
                | Self::Build70450AvailableEmptyWmoGeometryV1
                | Self::Build70450AvailableUnselectedWmoGeometryV1
                | Self::Build70450MaterialFilteredWmoGeometryV1
                | Self::Build70450DependencyHeavyMaterialFilteredWmoGeometryV1
                | Self::Build70450SourceHeavyMaterialFilteredWmoGeometryV1
                | Self::Build70450ReadHeavyMaterialFilteredWmoGeometryV1
                | Self::Build70450ModelHeavyMaterialFilteredWmoGeometryV1
                | Self::Build70450SourceExpandedMaterialFilteredWmoGeometryV1
                | Self::Build70450ModelOutputExpandedMaterialFilteredWmoGeometryV1
                | Self::Build70450ModelBytesExpandedMaterialFilteredWmoGeometryV1
                | Self::Build70450SourceExpandedV2MaterialFilteredWmoGeometryV1
                | Self::Build70450PlacementExpandedMaterialFilteredWmoGeometryV1
                | Self::Build70450AssemblyExpandedMaterialFilteredWmoGeometryV1
        ) {
            if plan.build != 70450 {
                return Err("product_plan_build");
            }
            if plan.schema_catalog != SchemaCatalog::WowDbDefs70450V1 {
                return Err("product_plan_schema_catalog");
            }
            if plan.components != (Component::Db2.bit() | Component::Collision.bit())
                || !matches!(plan.collision_storage, CollisionStorage::LeasedDiskV1(_))
                || plan.decryption != super::Decryption::Disabled
                || plan.map_domain.is_some()
                || plan.terrain != Selection::None
                || plan.navigation != Selection::None
                || plan.offmesh.is_some()
                || !plan.pack
                || plan.big_base_unit
                || plan.skip_liquids
                || plan.maximum_slope != 55
                || plan.non_steep_slope != 55
            {
                return Err("product_plan_component_scope");
            }
            if plan.db2.len() != 1 || plan.db2[0].name != "Map" {
                return Err("product_plan_dependencies");
            }
            return Ok(());
        }
        if self == Self::Build70450SelectedNavigationV1 {
            if plan.build != 70450 {
                return Err("product_plan_build");
            }
            if plan.schema_catalog != SchemaCatalog::WowDbDefs70450V1 {
                return Err("product_plan_schema_catalog");
            }
            if plan.components != Component::Navigation.bit()
                || !matches!(plan.collision_storage, CollisionStorage::Resident)
                || plan.decryption != super::Decryption::Disabled
                || plan.map_domain.is_some()
                || !plan.db2.is_empty()
                || plan.terrain != Selection::None
                || !matches!(plan.navigation, Selection::Map(_))
                || plan.offmesh.is_some()
                || !plan.pack
                || plan.big_base_unit
                || plan.skip_liquids
                || plan.maximum_slope != 55
                || plan.non_steep_slope != 55
            {
                return Err("product_plan_component_scope");
            }
            return Ok(());
        }
        if self == Self::Build70450AuthenticatedTerrainV1 {
            if plan.build != 70450 {
                return Err("product_plan_build");
            }
            if plan.schema_catalog != SchemaCatalog::WowDbDefs70450V1 {
                return Err("product_plan_schema_catalog");
            }
            if plan.components != (Component::Db2.bit() | Component::Terrain.bit())
                || !matches!(plan.collision_storage, CollisionStorage::Resident)
                || plan.decryption != super::Decryption::Disabled
                || plan.map_domain.is_some()
                || plan.navigation != Selection::None
                || plan.offmesh.is_some()
            {
                return Err("product_plan_component_scope");
            }
            if plan.db2.len() != 1 || plan.db2[0].name != "Map" {
                return Err("product_plan_dependencies");
            }
            if !matches!(plan.terrain, Selection::All | Selection::Map(_)) {
                return Err("product_terrain_selection");
            }
            return Ok(());
        }
        if self == Self::Build70450AuthenticatedTerrainPartialMapV1 {
            if plan.build != 70450 {
                return Err("product_plan_build");
            }
            if plan.schema_catalog != SchemaCatalog::WowDbDefs70450V1 {
                return Err("product_plan_schema_catalog");
            }
            if plan.components != (Component::Db2.bit() | Component::Terrain.bit())
                || !matches!(plan.collision_storage, CollisionStorage::Resident)
                || plan.decryption != super::Decryption::Disabled
                || plan.map_domain.is_some()
                || plan.navigation != Selection::None
                || plan.offmesh.is_some()
                || !plan.pack
                || plan.big_base_unit
                || plan.skip_liquids
                || plan.maximum_slope != 55
                || plan.non_steep_slope != 55
            {
                return Err("product_plan_component_scope");
            }
            if plan.db2.len() != 1 || plan.db2[0].name != "Map" {
                return Err("product_plan_dependencies");
            }
            if !matches!(plan.terrain, Selection::All | Selection::Map(_)) {
                return Err("product_terrain_selection");
            }
            return Ok(());
        }
        if self == Self::Build70450PartialCatalogTerrainV1 {
            if plan.build != 70450 {
                return Err("product_plan_build");
            }
            if plan.schema_catalog != SchemaCatalog::WowDbDefs70450V1 {
                return Err("product_plan_schema_catalog");
            }
            if plan.components != (Component::Db2.bit() | Component::Terrain.bit())
                || !matches!(plan.collision_storage, CollisionStorage::Resident)
                || plan.decryption != super::Decryption::Disabled
                || plan.map_domain.is_some()
                || plan.navigation != Selection::None
                || plan.offmesh.is_some()
                || !plan.pack
                || plan.big_base_unit
                || plan.skip_liquids
                || plan.maximum_slope != 55
                || plan.non_steep_slope != 55
            {
                return Err("product_plan_component_scope");
            }
            if plan.db2.is_empty()
                || plan.db2.len() > 1161
                || plan.db2.iter().filter(|entry| entry.name == "Map").count() != 1
            {
                return Err("product_plan_dependencies");
            }
            if !matches!(plan.terrain, Selection::All | Selection::Map(_)) {
                return Err("product_terrain_selection");
            }
            return Ok(());
        }
        if matches!(
            self,
            Self::Build70450AvailableNavigationV2
                | Self::Build70450AvailableNavigationV3
                | Self::Build70450AvailableNavigationV4
                | Self::Build70450AvailableNavigationV5
                | Self::Build70450AvailableNavigationV6
        ) {
            if plan.build != 70450 {
                return Err("product_plan_build");
            }
            if plan.schema_catalog != SchemaCatalog::WowDbDefs70450V1 {
                return Err("product_plan_schema_catalog");
            }
            if plan.components != Component::Navigation.bit()
                || !matches!(plan.collision_storage, CollisionStorage::Resident)
                || plan.decryption != super::Decryption::Disabled
                || plan.map_domain.is_some()
                || plan.terrain != Selection::None
                || plan.navigation != Selection::All
                || plan.offmesh.is_some()
                || !plan.pack
                || plan.big_base_unit
                || plan.skip_liquids
                || plan.maximum_slope != 55
                || plan.non_steep_slope != 55
            {
                return Err("product_plan_component_scope");
            }
            if !plan.db2.is_empty() {
                return Err("product_plan_dependencies");
            }
            return Ok(());
        }
        let (build, catalog, components) = match self {
            Self::Legacy
            | Self::Build70450AvailableNavigationV1
            | Self::Build70425CompatibleCollisionV1
            | Self::Build70450CompatibleCollisionV1
            | Self::Build70450CollisionProvidedKeysV1
            | Self::Build70450Db2PartialV1
            | Self::Build70450PartialCatalogTerrainV1
            | Self::Build70450CompatibleTerrainV1
            | Self::Build70450NavigationV1
            | Self::Build70450SelectedGeometryV1
            | Self::Build70450AuthenticatedSelectedGeometryV1
            | Self::Build70450AuthenticatedAvailableGeometryV1
            | Self::Build70450AvailableRootAbsentGeometryV1
            | Self::Build70450AvailableModelRootAbsentGeometryV1
            | Self::Build70450AvailableModelRootAbsentRequestCapacityGeometryV1
            | Self::Build70450AvailableVacatedWmoGeometryV1
            | Self::Build70450AvailableEmptyWmoGeometryV1
            | Self::Build70450AvailableUnselectedWmoGeometryV1
            | Self::Build70450MaterialFilteredWmoGeometryV1
            | Self::Build70450DependencyHeavyMaterialFilteredWmoGeometryV1
            | Self::Build70450SourceHeavyMaterialFilteredWmoGeometryV1
            | Self::Build70450ReadHeavyMaterialFilteredWmoGeometryV1
            | Self::Build70450ModelHeavyMaterialFilteredWmoGeometryV1
            | Self::Build70450SourceExpandedMaterialFilteredWmoGeometryV1
            | Self::Build70450ModelOutputExpandedMaterialFilteredWmoGeometryV1
            | Self::Build70450ModelBytesExpandedMaterialFilteredWmoGeometryV1
            | Self::Build70450SourceExpandedV2MaterialFilteredWmoGeometryV1
            | Self::Build70450PlacementExpandedMaterialFilteredWmoGeometryV1
            | Self::Build70450AssemblyExpandedMaterialFilteredWmoGeometryV1
            | Self::Build70450GameObjectExpandedMaterialFilteredWmoGeometryV1
            | Self::Build70450GameObjectMissingRootMaterialFilteredWmoGeometryV1
            | Self::Build70450GameObjectRootAndEncryptedUnavailableMaterialFilteredWmoGeometryV1
            | Self::Build70450LiquidObjectCompleteSourceV1
            | Self::Build70450LiquidTerrainVmapV1
            | Self::Build70450LiquidTerrainVmapCombinedV1
            | Self::Build70450Db2TerrainLiquidVmapV1
            | Self::Build70450SelectedNavigationV1 => {
                unreachable!("profile returned above")
            }
            Self::Build70450AvailableNavigationV2
            | Self::Build70450AvailableNavigationV3
            | Self::Build70450AvailableNavigationV4
            | Self::Build70450AvailableNavigationV5
            | Self::Build70450AvailableNavigationV6 => {
                unreachable!("profile returned above")
            }
            Self::Build70450AuthenticatedTerrainV1 => unreachable!("profile returned above"),
            Self::Build70450AuthenticatedTerrainPartialMapV1 => {
                unreachable!("profile returned above")
            }
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
            Self::Build70374AvailableTerrainV1 | Self::Build70374CompatibleTerrainV1 => (
                70374,
                SchemaCatalog::WowDbDefs70374V1,
                Component::Terrain.bit(),
            ),
            Self::Build70374Db2OnlyV1 => {
                (70374, SchemaCatalog::WowDbDefs70374V1, Component::Db2.bit())
            }
            Self::Build70450Db2OnlyV1 => {
                (70450, SchemaCatalog::WowDbDefs70450V1, Component::Db2.bit())
            }
            Self::Build70450Db2ProvidedKeysV1 => {
                (70450, SchemaCatalog::WowDbDefs70450V1, Component::Db2.bit())
            }
            Self::Build70450Db2GtCamerasV1 => (
                70450,
                SchemaCatalog::WowDbDefs70450V1,
                Component::Db2.bit() | Component::GameTables.bit() | Component::Cameras.bit(),
            ),
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
        if matches!(
            self,
            Self::Build70374AvailableTerrainV1 | Self::Build70374CompatibleTerrainV1
        ) {
            if plan.decryption != super::Decryption::Disabled
                || !plan.db2.is_empty()
                || !matches!(plan.terrain, Selection::All | Selection::Map(_))
                || plan.navigation != Selection::None
            {
                return Err("product_plan_dependencies");
            }
            return Ok(());
        }
        if plan.terrain != Selection::None || plan.navigation != Selection::None {
            return Err("product_plan_dependencies");
        }
        if self == Self::Build70450Db2OnlyV1 && plan.decryption != super::Decryption::Disabled {
            return Err("product_plan_component_scope");
        }
        if self == Self::Build70450Db2ProvidedKeysV1 {
            if plan.decryption != super::Decryption::ProvidedSalsa20V1 {
                return Err("product_plan_component_scope");
            }
        } else if self == Self::Build70450Db2GtCamerasV1
            && plan.decryption != super::Decryption::Disabled
        {
            return Err("product_plan_component_scope");
        }
        Ok(())
    }
}
