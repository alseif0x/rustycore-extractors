// SPDX-License-Identifier: GPL-3.0-or-later
// Modified by the RustyCore Extractors project, 2026-10-19.
// Authored 2026-10-11: explicit, canonical offline product selections.
use forever_casc::{Locale, local_index, package::Component, root};
use forever_db2::catalog;
use std::{collections::BTreeSet, fmt::Write};

mod publication;
pub use publication::PublicationProfile;
mod disk;
pub use disk::{CollisionStorage, DiskBudget};
mod offmesh;
pub use offmesh::{MAX_OFFMESH_CONNECTIONS, OFFMESH_PROFILE, OffMeshConnection};
mod map_domain;
pub use map_domain::{
    GAMEOBJECT_SCOPE_PROFILE, MAP_DOMAIN_PROFILE, MAP_DOMAIN_VERIFICATION_PROFILE,
    MAP_DOMAIN_VERIFY_SCOPE, PLACEMENT_ID_DOMAIN_PROFILE,
};

pub const AVAILABLE_TERRAIN_VERIFY_PROFILE: &str = "linux70374-available-map-terrain-product-v1";
pub const AVAILABLE_TERRAIN_VERIFY_SCOPE: &str =
    "artifact_integrity_intrinsic_formats_and_declared_available_map_coverage_links";

pub const COMPATIBLE_TERRAIN_VERIFY_PROFILE: &str =
    "linux70374-compatible-available-map-terrain-product-v1";
pub const COMPATIBLE_TERRAIN_VERIFY_SCOPE: &str =
    "artifact_integrity_intrinsic_formats_and_declared_available_map_omission_coverage_links";
pub const COMPATIBLE_TERRAIN_VERIFY_PROFILE_70450: &str =
    "linux70450-compatible-available-map-terrain-product-v1";
pub const COMPATIBLE_TERRAIN_VERIFY_SCOPE_70450: &str =
    "artifact_integrity_intrinsic_formats_and_declared_available_map_omission_coverage_links";

pub const MAX_PLAN_BYTES: usize = 128 * 1024;
pub const MAX_DB2_ENTRIES: usize = 1126;
pub const MAX_70205_DB2_ENTRIES: usize = 1161;
pub const MAX_70338_DB2_ENTRIES: usize = 1161;
pub const MAX_70374_DB2_ENTRIES: usize = 1161;
pub const MAX_70425_DB2_ENTRIES: usize = 1161;
pub const MAX_70450_DB2_ENTRIES: usize = 1161;
pub const COMPATIBLE_COLLISION_VERIFY_PROFILE: &str = "linux70425-compatible-gnu15-vmap-product-v1";
pub const COMPATIBLE_COLLISION_VERIFY_PROFILE_70450: &str =
    "linux70450-compatible-gnu15-vmap-product-v1";
pub const COMPATIBLE_COLLISION_VERIFY_SCOPE: &str = "artifact_integrity_complete_declared_vmap_associations_and_producer_reported_available_source_coverage";
pub const SELECTED_GEOMETRY_COLLISION_PROFILE: &str =
    "linux70450-selected-map-geometry-vmap-product-v1";
pub const SELECTED_GEOMETRY_COVERAGE_PROFILE: &str = "linux70450-selected-map-geometry-coverage-v1";
pub const SELECTED_GEOMETRY_VERIFY_PROFILE: &str =
    "linux70450-selected-map-geometry-verification-v1";
pub const SELECTED_GEOMETRY_VERIFY_SCOPE: &str =
    "artifact_integrity_selected_map_geometry_associations_and_declared_partial_source_coverage";
pub const SELECTED_GEOMETRY_GAMEOBJECT_SCOPE: &str = "excluded-by-profile-v1";
pub const SELECTED_GEOMETRY_MODEL_SCOPE: &str = "placement-only-v1";
pub const AUTHENTICATED_TERRAIN_VERIFY_PROFILE: &str =
    "linux70450-authenticated-available-map-terrain-product-v1";
pub const AUTHENTICATED_TERRAIN_VERIFY_SCOPE: &str =
    "artifact_integrity_and_terrain_coverage_independently_derived_from_sealed_map_db2";
pub const AUTHENTICATED_PARTIAL_MAP_VERIFY_PROFILE: &str =
    "linux70450-authenticated-available-map-terrain-partial-map-product-v1";
pub const AUTHENTICATED_PARTIAL_MAP_VERIFY_SCOPE: &str =
    "artifact_integrity_and_terrain_coverage_with_declared_partial_map_section_verification";
pub const PARTIAL_CATALOG_TERRAIN_VERIFY_PROFILE: &str =
    "linux70450-partial-catalog-authenticated-map-terrain-product-v1";
pub const PARTIAL_CATALOG_TERRAIN_VERIFY_SCOPE: &str =
    "artifact_integrity_selected_db2_and_authenticated_terrain_with_per_table_partial_sections";
pub const AUTHENTICATED_SELECTED_GEOMETRY_VERIFY_PROFILE: &str =
    "linux70450-authenticated-selected-map-geometry-verification-v1";
pub const AUTHENTICATED_SELECTED_GEOMETRY_VERIFY_SCOPE: &str = "artifact_integrity_selected_map_geometry_associations_and_independently_derived_map_vmap_domain_coverage";
pub const AVAILABLE_GEOMETRY_COLLISION_PROFILE: &str =
    "linux70450-authenticated-available-map-geometry-vmap-product-v1";
pub const AVAILABLE_GEOMETRY_COVERAGE_PROFILE: &str =
    "linux70450-available-map-geometry-coverage-v2";
pub const AUTHENTICATED_AVAILABLE_GEOMETRY_VERIFY_PROFILE: &str =
    "linux70450-authenticated-available-map-geometry-verification-v1";
pub const AUTHENTICATED_AVAILABLE_GEOMETRY_VERIFY_SCOPE: &str = "artifact_integrity_available_map_geometry_associations_and_independently_derived_map_vmap_domain_coverage";
pub const ROOT_ABSENT_GEOMETRY_COLLISION_PROFILE: &str =
    "linux70450-available-map-root-omissions-vmap-product-v1";
pub const ROOT_ABSENT_GEOMETRY_COVERAGE_PROFILE: &str =
    "linux70450-available-map-root-omissions-coverage-v3";
pub const ROOT_ABSENT_GEOMETRY_VERIFY_PROFILE: &str =
    "linux70450-available-map-root-omissions-verification-v1";
pub const ROOT_ABSENT_GEOMETRY_VERIFY_SCOPE: &str = "artifact_integrity_available_map_geometry_root_omissions_and_independently_derived_map_vmap_domain_coverage";
pub const MODEL_ROOT_ABSENT_GEOMETRY_COLLISION_PROFILE: &str =
    "linux70450-available-map-model-root-omissions-vmap-product-v1";
pub const MODEL_ROOT_ABSENT_GEOMETRY_COVERAGE_PROFILE: &str =
    "linux70450-available-map-model-root-omissions-coverage-v4";
pub const MODEL_ROOT_ABSENT_GEOMETRY_VERIFY_PROFILE: &str =
    "linux70450-available-map-model-root-omissions-verification-v1";
pub const MODEL_ROOT_ABSENT_GEOMETRY_VERIFY_SCOPE: &str = "artifact_integrity_available_map_model_root_omissions_and_independently_derived_map_vmap_domain_coverage";
pub const MODEL_ROOT_ABSENT_REQUEST_CAPACITY_GEOMETRY_COLLISION_PROFILE: &str =
    "linux70450-available-map-model-root-omissions-request-capacity-vmap-product-v1";
pub const MODEL_ROOT_ABSENT_REQUEST_CAPACITY_GEOMETRY_COVERAGE_PROFILE: &str =
    "linux70450-available-map-model-root-omissions-request-capacity-coverage-v5";
pub const MODEL_ROOT_ABSENT_REQUEST_CAPACITY_GEOMETRY_VERIFY_PROFILE: &str =
    "linux70450-available-map-model-root-omissions-request-capacity-verification-v1";
pub const MODEL_ROOT_ABSENT_REQUEST_CAPACITY_GEOMETRY_VERIFY_SCOPE: &str = "artifact_integrity_available_map_model_root_omissions_request_capacity_and_independently_derived_map_vmap_domain_coverage";
pub const VACATED_WMO_COLLISION_PROFILE: &str = "linux70450-available-vacated-wmo-vmap-product-v1";
pub const VACATED_WMO_COVERAGE_PROFILE: &str = "linux70450-available-vacated-wmo-coverage-v6";
pub const VACATED_WMO_VERIFY_PROFILE: &str = "linux70450-available-vacated-wmo-verification-v1";
pub const VACATED_WMO_VERIFY_SCOPE: &str =
    "artifact_integrity_available_vacated_wmo_and_independently_derived_map_vmap_domain_coverage";
pub const EMPTY_WMO_COLLISION_PROFILE: &str = "linux70450-available-empty-wmo-vmap-product-v1";
pub const EMPTY_WMO_COVERAGE_PROFILE: &str = "linux70450-available-empty-wmo-coverage-v7";
pub const EMPTY_WMO_VERIFY_PROFILE: &str = "linux70450-available-empty-wmo-verification-v1";
pub const EMPTY_WMO_VERIFY_SCOPE: &str =
    "artifact_integrity_available_empty_wmo_and_independently_derived_map_vmap_domain_coverage";
pub const UNSELECTED_WMO_COLLISION_PROFILE: &str =
    "linux70450-available-unselected-wmo-vmap-product-v1";
pub const UNSELECTED_WMO_COVERAGE_PROFILE: &str = "linux70450-available-unselected-wmo-coverage-v8";
pub const UNSELECTED_WMO_VERIFY_PROFILE: &str =
    "linux70450-available-unselected-wmo-verification-v1";
pub const UNSELECTED_WMO_VERIFY_SCOPE: &str = "artifact_integrity_available_unselected_wmo_and_independently_derived_map_vmap_domain_coverage";
pub const MATERIAL_FILTERED_WMO_COLLISION_PROFILE: &str =
    "linux70450-material-filtered-wmo-vmap-product-v1";
pub const MATERIAL_FILTERED_WMO_COVERAGE_PROFILE: &str =
    "linux70450-material-filtered-wmo-coverage-v9";
pub const MATERIAL_FILTERED_WMO_VERIFY_PROFILE: &str =
    "linux70450-material-filtered-wmo-verification-v1";
pub const MATERIAL_FILTERED_WMO_VERIFY_SCOPE: &str =
    "artifact_integrity_material_filtered_wmo_and_independently_derived_map_vmap_domain_coverage";
pub const DEPENDENCY_HEAVY_MATERIAL_FILTERED_WMO_COLLISION_PROFILE: &str =
    "linux70450-dependency-heavy-material-filtered-wmo-vmap-product-v1";
pub const DEPENDENCY_HEAVY_MATERIAL_FILTERED_WMO_COVERAGE_PROFILE: &str =
    "linux70450-dependency-heavy-material-filtered-wmo-coverage-v10";
pub const DEPENDENCY_HEAVY_MATERIAL_FILTERED_WMO_VERIFY_PROFILE: &str =
    "linux70450-dependency-heavy-material-filtered-wmo-verification-v1";
pub const DEPENDENCY_HEAVY_MATERIAL_FILTERED_WMO_VERIFY_SCOPE: &str = "artifact_integrity_dependency_heavy_material_filtered_wmo_and_independently_derived_map_vmap_domain_coverage";
pub const SOURCE_HEAVY_MATERIAL_FILTERED_WMO_COLLISION_PROFILE: &str =
    "linux70450-source-heavy-material-filtered-wmo-vmap-product-v1";
pub const SOURCE_HEAVY_MATERIAL_FILTERED_WMO_COVERAGE_PROFILE: &str =
    "linux70450-source-heavy-material-filtered-wmo-coverage-v11";
pub const SOURCE_HEAVY_MATERIAL_FILTERED_WMO_VERIFY_PROFILE: &str =
    "linux70450-source-heavy-material-filtered-wmo-verification-v1";
pub const SOURCE_HEAVY_MATERIAL_FILTERED_WMO_VERIFY_SCOPE: &str = "artifact_integrity_source_heavy_material_filtered_wmo_and_independently_derived_map_vmap_domain_coverage";
pub const READ_HEAVY_MATERIAL_FILTERED_WMO_COLLISION_PROFILE: &str =
    "linux70450-read-heavy-material-filtered-wmo-vmap-product-v1";
pub const READ_HEAVY_MATERIAL_FILTERED_WMO_COVERAGE_PROFILE: &str =
    "linux70450-read-heavy-material-filtered-wmo-coverage-v12";
pub const READ_HEAVY_MATERIAL_FILTERED_WMO_VERIFY_PROFILE: &str =
    "linux70450-read-heavy-material-filtered-wmo-verification-v1";
pub const READ_HEAVY_MATERIAL_FILTERED_WMO_VERIFY_SCOPE: &str = "artifact_integrity_read_heavy_material_filtered_wmo_and_independently_derived_map_vmap_domain_coverage";
pub const MODEL_HEAVY_MATERIAL_FILTERED_WMO_COLLISION_PROFILE: &str =
    "linux70450-model-heavy-material-filtered-wmo-vmap-product-v1";
pub const MODEL_HEAVY_MATERIAL_FILTERED_WMO_COVERAGE_PROFILE: &str =
    "linux70450-model-heavy-material-filtered-wmo-coverage-v13";
pub const MODEL_HEAVY_MATERIAL_FILTERED_WMO_VERIFY_PROFILE: &str =
    "linux70450-model-heavy-material-filtered-wmo-verification-v1";
pub const MODEL_HEAVY_MATERIAL_FILTERED_WMO_VERIFY_SCOPE: &str = "artifact_integrity_model_heavy_material_filtered_wmo_and_independently_derived_map_vmap_domain_coverage";
pub const SOURCE_EXPANDED_MATERIAL_FILTERED_WMO_COLLISION_PROFILE: &str =
    "linux70450-source-expanded-material-filtered-wmo-vmap-product-v1";
pub const SOURCE_EXPANDED_MATERIAL_FILTERED_WMO_COVERAGE_PROFILE: &str =
    "linux70450-source-expanded-material-filtered-wmo-coverage-v14";
pub const SOURCE_EXPANDED_MATERIAL_FILTERED_WMO_VERIFY_PROFILE: &str =
    "linux70450-source-expanded-material-filtered-wmo-verification-v1";
pub const SOURCE_EXPANDED_MATERIAL_FILTERED_WMO_VERIFY_SCOPE: &str = "artifact_integrity_source_expanded_material_filtered_wmo_and_independently_derived_map_vmap_domain_coverage";
pub const MODEL_OUTPUT_EXPANDED_MATERIAL_FILTERED_WMO_COLLISION_PROFILE: &str =
    "linux70450-model-output-expanded-material-filtered-wmo-vmap-product-v1";
pub const MODEL_OUTPUT_EXPANDED_MATERIAL_FILTERED_WMO_COVERAGE_PROFILE: &str =
    "linux70450-model-output-expanded-material-filtered-wmo-coverage-v15";
pub const MODEL_OUTPUT_EXPANDED_MATERIAL_FILTERED_WMO_VERIFY_PROFILE: &str =
    "linux70450-model-output-expanded-material-filtered-wmo-verification-v1";
pub const MODEL_OUTPUT_EXPANDED_MATERIAL_FILTERED_WMO_VERIFY_SCOPE: &str = "artifact_integrity_model_output_expanded_material_filtered_wmo_and_independently_derived_map_vmap_domain_coverage";
pub const MODEL_BYTES_EXPANDED_MATERIAL_FILTERED_WMO_COLLISION_PROFILE: &str =
    "linux70450-model-bytes-expanded-material-filtered-wmo-vmap-product-v1";
pub const MODEL_BYTES_EXPANDED_MATERIAL_FILTERED_WMO_COVERAGE_PROFILE: &str =
    "linux70450-model-bytes-expanded-material-filtered-wmo-coverage-v16";
pub const MODEL_BYTES_EXPANDED_MATERIAL_FILTERED_WMO_VERIFY_PROFILE: &str =
    "linux70450-model-bytes-expanded-material-filtered-wmo-verification-v1";
pub const MODEL_BYTES_EXPANDED_MATERIAL_FILTERED_WMO_VERIFY_SCOPE: &str = "artifact_integrity_model_bytes_expanded_material_filtered_wmo_and_independently_derived_map_vmap_domain_coverage";
pub const SOURCE_EXPANDED_V2_MATERIAL_FILTERED_WMO_COLLISION_PROFILE: &str =
    "linux70450-source-expanded-v2-material-filtered-wmo-vmap-product-v1";
pub const SOURCE_EXPANDED_V2_MATERIAL_FILTERED_WMO_COVERAGE_PROFILE: &str =
    "linux70450-source-expanded-v2-material-filtered-wmo-coverage-v17";
pub const SOURCE_EXPANDED_V2_MATERIAL_FILTERED_WMO_VERIFY_PROFILE: &str =
    "linux70450-source-expanded-v2-material-filtered-wmo-verification-v1";
pub const SOURCE_EXPANDED_V2_MATERIAL_FILTERED_WMO_VERIFY_SCOPE: &str = "artifact_integrity_source_expanded_v2_material_filtered_wmo_and_independently_derived_map_vmap_domain_coverage";
pub const PLACEMENT_EXPANDED_MATERIAL_FILTERED_WMO_COLLISION_PROFILE: &str =
    "linux70450-placement-expanded-material-filtered-wmo-vmap-product-v1";
pub const PLACEMENT_EXPANDED_MATERIAL_FILTERED_WMO_COVERAGE_PROFILE: &str =
    "linux70450-placement-expanded-material-filtered-wmo-coverage-v18";
pub const PLACEMENT_EXPANDED_MATERIAL_FILTERED_WMO_VERIFY_PROFILE: &str =
    "linux70450-placement-expanded-material-filtered-wmo-verification-v1";
pub const PLACEMENT_EXPANDED_MATERIAL_FILTERED_WMO_VERIFY_SCOPE: &str = "artifact_integrity_placement_expanded_material_filtered_wmo_and_independently_derived_map_vmap_domain_coverage";
pub const ASSEMBLY_EXPANDED_MATERIAL_FILTERED_WMO_COLLISION_PROFILE: &str =
    "linux70450-assembly-expanded-material-filtered-wmo-vmap-product-v1";
pub const ASSEMBLY_EXPANDED_MATERIAL_FILTERED_WMO_COVERAGE_PROFILE: &str =
    "linux70450-assembly-expanded-material-filtered-wmo-coverage-v19";
pub const ASSEMBLY_EXPANDED_MATERIAL_FILTERED_WMO_VERIFY_PROFILE: &str =
    "linux70450-assembly-expanded-material-filtered-wmo-verification-v1";
pub const ASSEMBLY_EXPANDED_MATERIAL_FILTERED_WMO_VERIFY_SCOPE: &str = "artifact_integrity_assembly_expanded_material_filtered_wmo_and_independently_derived_map_vmap_domain_coverage";
pub const GAMEOBJECT_EXPANDED_MATERIAL_FILTERED_WMO_COLLISION_PROFILE: &str =
    "linux70450-gameobject-expanded-material-filtered-wmo-vmap-product-v1";
pub const GAMEOBJECT_EXPANDED_MATERIAL_FILTERED_WMO_COVERAGE_PROFILE: &str =
    "linux70450-gameobject-expanded-material-filtered-wmo-coverage-v20";
pub const GAMEOBJECT_EXPANDED_MATERIAL_FILTERED_WMO_VERIFY_PROFILE: &str =
    "linux70450-gameobject-expanded-material-filtered-wmo-verification-v1";
pub const GAMEOBJECT_EXPANDED_MATERIAL_FILTERED_WMO_VERIFY_SCOPE: &str = "artifact_integrity_gameobject_expanded_material_filtered_wmo_and_independently_derived_map_godi_vmap_domain_coverage";
pub const GAMEOBJECT_EXPANDED_MATERIAL_FILTERED_WMO_GAMEOBJECT_SCOPE: &str =
    "available-fdid-direct-v1";
pub const GAMEOBJECT_EXPANDED_MATERIAL_FILTERED_WMO_MODEL_SCOPE: &str =
    "placement-and-gameobject-v1";
pub const GAMEOBJECT_MISSING_ROOT_MATERIAL_FILTERED_WMO_COLLISION_PROFILE: &str =
    "linux70450-gameobject-missing-root-material-filtered-wmo-vmap-product-v1";
pub const GAMEOBJECT_MISSING_ROOT_MATERIAL_FILTERED_WMO_COVERAGE_PROFILE: &str =
    "linux70450-gameobject-missing-root-material-filtered-wmo-coverage-v21";
pub const GAMEOBJECT_MISSING_ROOT_MATERIAL_FILTERED_WMO_VERIFY_PROFILE: &str =
    "linux70450-gameobject-missing-root-material-filtered-wmo-verification-v1";
pub const GAMEOBJECT_MISSING_ROOT_MATERIAL_FILTERED_WMO_VERIFY_SCOPE: &str = "artifact_integrity_gameobject_missing_root_material_filtered_wmo_and_independently_derived_map_godi_vmap_domain_coverage";
pub const GAMEOBJECT_MISSING_ROOT_MATERIAL_FILTERED_WMO_GAMEOBJECT_SCOPE: &str =
    "available-fdid-direct-missing-root-v1";
pub const GAMEOBJECT_MISSING_ROOT_MATERIAL_FILTERED_WMO_MODEL_SCOPE: &str =
    "placement-and-gameobject-with-missing-root-v1";
pub const GAMEOBJECT_ROOT_ENCRYPTED_UNAVAILABLE_MATERIAL_FILTERED_WMO_COLLISION_PROFILE: &str =
    "linux70450-gameobject-root-encrypted-unavailable-material-filtered-wmo-vmap-product-v1";
pub const GAMEOBJECT_ROOT_ENCRYPTED_UNAVAILABLE_MATERIAL_FILTERED_WMO_COVERAGE_PROFILE: &str =
    "linux70450-gameobject-root-encrypted-unavailable-material-filtered-wmo-coverage-v22";
pub const GAMEOBJECT_ROOT_ENCRYPTED_UNAVAILABLE_MATERIAL_FILTERED_WMO_VERIFY_PROFILE: &str =
    "linux70450-gameobject-root-encrypted-unavailable-material-filtered-wmo-verification-v1";
pub const GAMEOBJECT_ROOT_ENCRYPTED_UNAVAILABLE_MATERIAL_FILTERED_WMO_VERIFY_SCOPE: &str = "artifact_integrity_gameobject_root_and_encrypted_unavailable_material_filtered_wmo_and_independently_derived_map_godi_vmap_domain_coverage";
pub const GAMEOBJECT_ROOT_ENCRYPTED_UNAVAILABLE_MATERIAL_FILTERED_WMO_GAMEOBJECT_SCOPE: &str =
    "available-fdid-direct-root-absent-or-encrypted-unavailable-v1";
pub const GAMEOBJECT_ROOT_ENCRYPTED_UNAVAILABLE_MATERIAL_FILTERED_WMO_MODEL_SCOPE: &str =
    "placement-and-gameobject-with-root-absent-or-encrypted-unavailable-v1";
pub const LIQUID_OBJECT_COMPLETE_SOURCE_PROFILE: &str =
    "linux70450-liquid-object-complete-source-vmap-product-v1";
pub const LIQUID_OBJECT_COMPLETE_SOURCE_COVERAGE_PROFILE: &str =
    "linux70450-liquid-object-complete-source-coverage-v23";
pub const LIQUID_OBJECT_COMPLETE_SOURCE_CODEC_PROFILE: &str =
    "linux70450-liquid-object-complete-source-v1";
pub const LIQUID_OBJECT_COMPLETE_SOURCE_DOMAIN_PROFILE: &str = "liquid-object-complete-source-v1";
pub const LIQUID_OBJECT_COMPLETE_SOURCE_VERIFY_PROFILE: &str =
    "linux70450-liquid-object-complete-source-verification-v1";
pub const LIQUID_OBJECT_COMPLETE_SOURCE_VERIFY_SCOPE: &str =
    "sealed_liquid_object_source_facts_and_v49_available_map_geometry";
pub const LIQUID_TERRAIN_VMAP_PRODUCT_PROFILE: &str = "linux70450-liquid-terrain-vmap-product-v1";
pub const LIQUID_TERRAIN_VMAP_COMPLETION_PROFILE: &str =
    "linux70450-liquid-terrain-vmap-completion-v1";
pub const LIQUID_TERRAIN_VMAP_VERIFY_PROFILE: &str =
    "linux70450-liquid-terrain-vmap-verification-v1";
pub const LIQUID_TERRAIN_VMAP_VERIFY_SCOPE: &str =
    "authenticated_all_available_map_terrain_and_v49_geometry_with_complete_liquid_sources";
pub const DB2_TERRAIN_LIQUID_VMAP_PRODUCT_PROFILE: &str =
    "linux70450-db2-terrain-liquid-vmap-product-v1";
pub const DB2_TERRAIN_LIQUID_VMAP_COMPLETION_PROFILE: &str =
    "linux70450-db2-terrain-liquid-vmap-completion-v1";
pub const DB2_TERRAIN_LIQUID_VMAP_COVERAGE_PROFILE: &str =
    "linux70450-db2-terrain-liquid-vmap-coverage-v1";
pub const DB2_TERRAIN_LIQUID_VMAP_VERIFY_PROFILE: &str =
    "linux70450-db2-terrain-liquid-vmap-verification-v1";
pub const DB2_TERRAIN_LIQUID_VMAP_VERIFY_SCOPE: &str =
    "authenticated_all_available_map_terrain_and_v50_geometry_with_complete_liquid_sources";
pub const DB2_TERRAIN_LIQUID_VMAP_CODEC_PROFILE: &str =
    "linux70450-db2-terrain-liquid-vmap-geometry-v1";
pub const DB2_TERRAIN_LIQUID_VMAP_DOMAIN_PROFILE: &str =
    "db2-terrain-liquid-vmap-available-map-domain-v1";

/// Explicit DB2 schema source. Existing product plans retain their legacy
/// pinned schema; builds 70205 and 70338 plans opt into separately pinned
/// metadata catalogs without relabeling older package identities.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum SchemaCatalog {
    #[default]
    PinnedMetadataV1,
    WowDbDefs70205V1,
    WowDbDefs70338V1,
    WowDbDefs70374V1,
    WowDbDefs70425V1,
    WowDbDefs70450V1,
}

impl SchemaCatalog {
    pub const fn id(self) -> &'static str {
        match self {
            Self::PinnedMetadataV1 => catalog::CATALOG_ID,
            Self::WowDbDefs70205V1 => catalog::WOWDBDEFS_70205_CATALOG_ID,
            Self::WowDbDefs70338V1 => catalog::WOWDBDEFS_70338_CATALOG_ID,
            Self::WowDbDefs70374V1 => catalog::WOWDBDEFS_70374_CATALOG_ID,
            Self::WowDbDefs70425V1 => catalog::WOWDBDEFS_70425_CATALOG_ID,
            Self::WowDbDefs70450V1 => catalog::WOWDBDEFS_70450_CATALOG_ID,
        }
    }

    const fn max_entries(self) -> usize {
        match self {
            Self::PinnedMetadataV1 => MAX_DB2_ENTRIES,
            Self::WowDbDefs70205V1 => MAX_70205_DB2_ENTRIES,
            Self::WowDbDefs70338V1 => MAX_70338_DB2_ENTRIES,
            Self::WowDbDefs70374V1 => MAX_70374_DB2_ENTRIES,
            Self::WowDbDefs70425V1 => MAX_70425_DB2_ENTRIES,
            Self::WowDbDefs70450V1 => MAX_70450_DB2_ENTRIES,
        }
    }
}

/// An explicit decoding capability, without key material or keyset identity.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Decryption {
    Disabled,
    ProvidedSalsa20V1,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Entry {
    pub name: String,
    pub id: u32,
}

/// Terrain uses x=column/y=row. Navigation adapters must use row=y/column=x.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Selection {
    None,
    All,
    Map(u32),
    Tile { map: u32, x: u8, y: u8 },
}
impl Selection {
    fn parse(value: &str) -> Result<Self, &'static str> {
        let fields: Vec<_> = value.split(':').collect();
        match fields.as_slice() {
            ["none"] => Ok(Self::None),
            ["all"] => Ok(Self::All),
            ["map", map] => Ok(Self::Map(decimal(map)?)),
            ["tile", map, x, y] => {
                let x = decimal(x)?;
                let y = decimal(y)?;
                if x >= 64 || y >= 64 {
                    return Err("product_plan_selection");
                }
                Ok(Self::Tile {
                    map: decimal(map)?,
                    x: x as u8,
                    y: y as u8,
                })
            }
            _ => Err("product_plan_selection"),
        }
    }
    fn render(self) -> String {
        match self {
            Self::None => "none".into(),
            Self::All => "all".into(),
            Self::Map(map) => format!("map:{map}"),
            Self::Tile { map, x, y } => format!("tile:{map}:{x}:{y}"),
        }
    }
    fn validate(self) -> Result<(), &'static str> {
        if matches!(self, Self::Tile { x, y, .. } if x >= 64 || y >= 64) {
            Err("product_plan_selection")
        } else {
            Ok(())
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Plan {
    /// Explicit component publication contract; old plans retain capability inference.
    pub publication_profile: PublicationProfile,
    pub build: u32,
    pub schema_catalog: SchemaCatalog,
    pub locale: Locale,
    pub index_layout: local_index::Layout,
    pub root_layout: root::Layout,
    pub decryption: Decryption,
    pub collision_storage: CollisionStorage,
    /// Some explicitly selects Linux v8; v7 is reserved for the deferred draft.
    pub map_domain: Option<u32>,
    pub components: u8,
    pub db2: Vec<Entry>,
    pub terrain: Selection,
    pub navigation: Selection,
    pub pack: bool,
    pub big_base_unit: bool,
    pub skip_liquids: bool,
    pub maximum_slope: u8,
    pub non_steep_slope: u8,
    /// None retains v1-v5; Some (including empty) explicitly selects v6.
    pub offmesh: Option<Vec<OffMeshConnection>>,
}

impl Plan {
    /// Parse v1-v6 and the explicit Linux v8 grammar; v7 remains reserved.
    /// Exact re-rendering closes spelling,
    /// ordering, whitespace, numeric representation and final-newline aliases.
    pub fn parse(input: &str) -> Result<Self, &'static str> {
        if input.len() > MAX_PLAN_BYTES {
            return Err("product_plan_size");
        }
        if input.starts_with("forever-product\t57\n") {
            let legacy = input
                .replacen("forever-product\t57\n", "forever-product\t23\n", 1)
                .replacen("navigation\tall-available-maps\n", "navigation\tmap:0\n", 1);
            let mut plan = Self::parse(&legacy)?;
            plan.publication_profile = PublicationProfile::Build70450AvailableNavigationV6;
            plan.navigation = Selection::All;
            plan.validate()?;
            if plan.render() != input {
                return Err("product_plan_noncanonical");
            }
            return Ok(plan);
        }
        if input.starts_with("forever-product\t56\n") {
            let legacy = input
                .replacen("forever-product\t56\n", "forever-product\t23\n", 1)
                .replacen("navigation\tall-available-maps\n", "navigation\tmap:0\n", 1);
            let mut plan = Self::parse(&legacy)?;
            plan.publication_profile = PublicationProfile::Build70450AvailableNavigationV5;
            plan.navigation = Selection::All;
            plan.validate()?;
            if plan.render() != input {
                return Err("product_plan_noncanonical");
            }
            return Ok(plan);
        }
        if input.starts_with("forever-product\t55\n") {
            let legacy = input
                .replacen("forever-product\t55\n", "forever-product\t23\n", 1)
                .replacen("navigation\tall-available-maps\n", "navigation\tmap:0\n", 1);
            let mut plan = Self::parse(&legacy)?;
            plan.publication_profile = PublicationProfile::Build70450AvailableNavigationV4;
            plan.navigation = Selection::All;
            plan.validate()?;
            if plan.render() != input {
                return Err("product_plan_noncanonical");
            }
            return Ok(plan);
        }
        if input.starts_with("forever-product\t54\n") {
            let legacy = input
                .replacen("forever-product\t54\n", "forever-product\t23\n", 1)
                .replacen("navigation\tall-available-maps\n", "navigation\tmap:0\n", 1);
            let mut plan = Self::parse(&legacy)?;
            plan.publication_profile = PublicationProfile::Build70450AvailableNavigationV3;
            plan.navigation = Selection::All;
            plan.validate()?;
            if plan.render() != input {
                return Err("product_plan_noncanonical");
            }
            return Ok(plan);
        }
        if input.starts_with("forever-product\t53\n") {
            let legacy = input
                .replacen("forever-product\t53\n", "forever-product\t23\n", 1)
                .replacen("navigation\tall-available-maps\n", "navigation\tmap:0\n", 1);
            let mut plan = Self::parse(&legacy)?;
            plan.publication_profile = PublicationProfile::Build70450AvailableNavigationV2;
            plan.navigation = Selection::All;
            plan.validate()?;
            if plan.render() != input {
                return Err("product_plan_noncanonical");
            }
            return Ok(plan);
        }
        if input.starts_with("forever-product\t52\n") {
            let legacy = input
                .replacen("forever-product\t52\n", "forever-product\t23\n", 1)
                .replacen("navigation\tall-available-maps\n", "navigation\tmap:0\n", 1);
            let mut plan = Self::parse(&legacy)?;
            plan.publication_profile = PublicationProfile::Build70450AvailableNavigationV1;
            plan.navigation = Selection::All;
            plan.validate()?;
            if plan.render() != input {
                return Err("product_plan_noncanonical");
            }
            return Ok(plan);
        }
        if input.len() > MAX_PLAN_BYTES {
            return Err("product_plan_size");
        }
        if !input.is_ascii()
            || input
                .bytes()
                .any(|b| !(b == b'\n' || b == b'\t' || (32..=126).contains(&b)))
        {
            return Err("product_plan_format");
        }
        let mut lines = input.split_terminator('\n');
        let version = match field(&mut lines, "forever-product")? {
            "1" => 1,
            "2" => 2,
            "3" => 3,
            "4" => 4,
            "5" => 5,
            "6" => 6,
            "8" => 8,
            "9" => 9,
            "10" => 10,
            "11" => 11,
            "12" => 12,
            "13" => 13,
            "14" => 14,
            "15" => 15,
            "16" => 16,
            "17" => 17,
            "18" => 18,
            "19" => 19,
            "20" => 20,
            "21" => 21,
            "22" => 22,
            "23" => 23,
            "24" => 24,
            "25" => 25,
            "26" => 26,
            "27" => 27,
            "28" => 28,
            "29" => 29,
            "30" => 30,
            "31" => 31,
            "32" => 32,
            "33" => 33,
            "34" => 34,
            "35" => 35,
            "36" => 36,
            "37" => 37,
            "38" => 38,
            "39" => 39,
            "40" => 40,
            "41" => 41,
            "42" => 42,
            "43" => 43,
            "44" => 44,
            "45" => 45,
            "46" => 46,
            "47" => 47,
            "48" => 48,
            "49" => 49,
            "50" => 50,
            "51" => 51,
            "52" => 52,
            "53" => 53,
            "54" => 54,
            "55" => 55,
            "56" => 56,
            "57" => 57,
            _ => return Err("product_plan_version"),
        };
        let build = decimal(field(&mut lines, "build")?)?;
        let locale = match field(&mut lines, "locale")? {
            "enUS" => Locale::EnUs,
            "esES" => Locale::EsEs,
            _ => return Err("product_plan_locale"),
        };
        let index_layout = match field(&mut lines, "index")? {
            "contiguous" => local_index::Layout::Contiguous,
            "paged" => local_index::Layout::Paged,
            _ => return Err("product_plan_index"),
        };
        let root_layout = match field(&mut lines, "root")? {
            "headerless-18125" => root::Layout::Headerless18125,
            "header-30080" => root::Layout::Header30080,
            "header-50893-v1" => root::Layout::Header50893V1,
            "header-50893-v2" => root::Layout::Header50893V2,
            _ => return Err("product_plan_root"),
        };
        let schema_catalog = match version {
            4 => match field(&mut lines, "schema-catalog")? {
                "wowdbdefs-70205-v1" => SchemaCatalog::WowDbDefs70205V1,
                _ => return Err("product_plan_schema_catalog"),
            },
            5 | 9 => match field(&mut lines, "schema-catalog")? {
                "wowdbdefs-70338-v1" => SchemaCatalog::WowDbDefs70338V1,
                _ => return Err("product_plan_schema_catalog"),
            },
            14 => match field(&mut lines, "schema-catalog")? {
                "wowdbdefs-70425-v1" => SchemaCatalog::WowDbDefs70425V1,
                _ => return Err("product_plan_schema_catalog"),
            },
            15..=57 => match field(&mut lines, "schema-catalog")? {
                "wowdbdefs-70450-v1" => SchemaCatalog::WowDbDefs70450V1,
                _ => return Err("product_plan_schema_catalog"),
            },
            10..=13 => match field(&mut lines, "schema-catalog")? {
                "wowdbdefs-70374-v1" => SchemaCatalog::WowDbDefs70374V1,
                _ => return Err("product_plan_schema_catalog"),
            },
            _ => SchemaCatalog::PinnedMetadataV1,
        };
        let decryption = match version {
            1 => Decryption::Disabled,
            2 => match field(&mut lines, "decryption")? {
                "provided-salsa20-v1" => Decryption::ProvidedSalsa20V1,
                _ => return Err("product_plan_decryption"),
            },
            3..=6 | 8..=57 => match field(&mut lines, "decryption")? {
                "disabled" => Decryption::Disabled,
                "provided-salsa20-v1" => Decryption::ProvidedSalsa20V1,
                _ => return Err("product_plan_decryption"),
            },
            _ => unreachable!("version was validated above"),
        };
        if (matches!(version, 25..=50) || version == 51)
            && field(&mut lines, "partial_encrypted_sections")
                .map_err(|_| "product_plan_partial_encrypted_sections")?
                != "declared-skip"
        {
            return Err("product_plan_partial_encrypted_sections");
        }
        let collision_storage = if matches!(version, 3 | 8 | 14 | 15 | 18 | 21 | 22 | 28..=51)
            || (matches!(version, 4..=6)
                && lines
                    .clone()
                    .next()
                    .is_some_and(|line| line.starts_with("collision-storage\t")))
        {
            CollisionStorage::parse(&mut lines)?
        } else {
            CollisionStorage::Resident
        };
        let map_domain = if matches!(version, 30..=51) {
            let (profile, coverage) = if version == 51 {
                (
                    DB2_TERRAIN_LIQUID_VMAP_PRODUCT_PROFILE,
                    DB2_TERRAIN_LIQUID_VMAP_COVERAGE_PROFILE,
                )
            } else if version == 50 {
                (
                    LIQUID_OBJECT_COMPLETE_SOURCE_PROFILE,
                    LIQUID_OBJECT_COMPLETE_SOURCE_COVERAGE_PROFILE,
                )
            } else if version == 49 {
                (
                    GAMEOBJECT_ROOT_ENCRYPTED_UNAVAILABLE_MATERIAL_FILTERED_WMO_COLLISION_PROFILE,
                    GAMEOBJECT_ROOT_ENCRYPTED_UNAVAILABLE_MATERIAL_FILTERED_WMO_COVERAGE_PROFILE,
                )
            } else if version == 48 {
                (
                    GAMEOBJECT_MISSING_ROOT_MATERIAL_FILTERED_WMO_COLLISION_PROFILE,
                    GAMEOBJECT_MISSING_ROOT_MATERIAL_FILTERED_WMO_COVERAGE_PROFILE,
                )
            } else if version == 47 {
                (
                    GAMEOBJECT_EXPANDED_MATERIAL_FILTERED_WMO_COLLISION_PROFILE,
                    GAMEOBJECT_EXPANDED_MATERIAL_FILTERED_WMO_COVERAGE_PROFILE,
                )
            } else if version == 46 {
                (
                    ASSEMBLY_EXPANDED_MATERIAL_FILTERED_WMO_COLLISION_PROFILE,
                    ASSEMBLY_EXPANDED_MATERIAL_FILTERED_WMO_COVERAGE_PROFILE,
                )
            } else if version == 45 {
                (
                    PLACEMENT_EXPANDED_MATERIAL_FILTERED_WMO_COLLISION_PROFILE,
                    PLACEMENT_EXPANDED_MATERIAL_FILTERED_WMO_COVERAGE_PROFILE,
                )
            } else if version == 44 {
                (
                    SOURCE_EXPANDED_V2_MATERIAL_FILTERED_WMO_COLLISION_PROFILE,
                    SOURCE_EXPANDED_V2_MATERIAL_FILTERED_WMO_COVERAGE_PROFILE,
                )
            } else if version == 43 {
                (
                    MODEL_BYTES_EXPANDED_MATERIAL_FILTERED_WMO_COLLISION_PROFILE,
                    MODEL_BYTES_EXPANDED_MATERIAL_FILTERED_WMO_COVERAGE_PROFILE,
                )
            } else if version == 42 {
                (
                    MODEL_OUTPUT_EXPANDED_MATERIAL_FILTERED_WMO_COLLISION_PROFILE,
                    MODEL_OUTPUT_EXPANDED_MATERIAL_FILTERED_WMO_COVERAGE_PROFILE,
                )
            } else if version == 41 {
                (
                    SOURCE_EXPANDED_MATERIAL_FILTERED_WMO_COLLISION_PROFILE,
                    SOURCE_EXPANDED_MATERIAL_FILTERED_WMO_COVERAGE_PROFILE,
                )
            } else if version == 40 {
                (
                    MODEL_HEAVY_MATERIAL_FILTERED_WMO_COLLISION_PROFILE,
                    MODEL_HEAVY_MATERIAL_FILTERED_WMO_COVERAGE_PROFILE,
                )
            } else if version == 39 {
                (
                    READ_HEAVY_MATERIAL_FILTERED_WMO_COLLISION_PROFILE,
                    READ_HEAVY_MATERIAL_FILTERED_WMO_COVERAGE_PROFILE,
                )
            } else if version == 38 {
                (
                    SOURCE_HEAVY_MATERIAL_FILTERED_WMO_COLLISION_PROFILE,
                    SOURCE_HEAVY_MATERIAL_FILTERED_WMO_COVERAGE_PROFILE,
                )
            } else if version == 37 {
                (
                    DEPENDENCY_HEAVY_MATERIAL_FILTERED_WMO_COLLISION_PROFILE,
                    DEPENDENCY_HEAVY_MATERIAL_FILTERED_WMO_COVERAGE_PROFILE,
                )
            } else if version == 36 {
                (
                    MATERIAL_FILTERED_WMO_COLLISION_PROFILE,
                    MATERIAL_FILTERED_WMO_COVERAGE_PROFILE,
                )
            } else if version == 35 {
                (
                    UNSELECTED_WMO_COLLISION_PROFILE,
                    UNSELECTED_WMO_COVERAGE_PROFILE,
                )
            } else if version == 34 {
                (EMPTY_WMO_COLLISION_PROFILE, EMPTY_WMO_COVERAGE_PROFILE)
            } else if version == 33 {
                (VACATED_WMO_COLLISION_PROFILE, VACATED_WMO_COVERAGE_PROFILE)
            } else if version == 32 {
                (
                    MODEL_ROOT_ABSENT_REQUEST_CAPACITY_GEOMETRY_COLLISION_PROFILE,
                    MODEL_ROOT_ABSENT_REQUEST_CAPACITY_GEOMETRY_COVERAGE_PROFILE,
                )
            } else if version == 31 {
                (
                    MODEL_ROOT_ABSENT_GEOMETRY_COLLISION_PROFILE,
                    MODEL_ROOT_ABSENT_GEOMETRY_COVERAGE_PROFILE,
                )
            } else {
                (
                    ROOT_ABSENT_GEOMETRY_COLLISION_PROFILE,
                    ROOT_ABSENT_GEOMETRY_COVERAGE_PROFILE,
                )
            };
            if field(&mut lines, "collision-profile")? != profile {
                return Err("product_plan_collision_profile");
            }
            if field(&mut lines, "collision-domain")? != "all-available-maps" {
                return Err("product_plan_collision_domain");
            }
            if field(&mut lines, "collision-coverage")? != coverage {
                return Err("product_plan_collision_coverage");
            }
            let (gameobject_scope, model_scope) = if version == 51 {
                (
                    GAMEOBJECT_ROOT_ENCRYPTED_UNAVAILABLE_MATERIAL_FILTERED_WMO_GAMEOBJECT_SCOPE,
                    GAMEOBJECT_ROOT_ENCRYPTED_UNAVAILABLE_MATERIAL_FILTERED_WMO_MODEL_SCOPE,
                )
            } else if version == 50 {
                ("source-only-v1", "source-only-v1")
            } else if version == 49 {
                (
                    GAMEOBJECT_ROOT_ENCRYPTED_UNAVAILABLE_MATERIAL_FILTERED_WMO_GAMEOBJECT_SCOPE,
                    GAMEOBJECT_ROOT_ENCRYPTED_UNAVAILABLE_MATERIAL_FILTERED_WMO_MODEL_SCOPE,
                )
            } else if version == 48 {
                (
                    GAMEOBJECT_MISSING_ROOT_MATERIAL_FILTERED_WMO_GAMEOBJECT_SCOPE,
                    GAMEOBJECT_MISSING_ROOT_MATERIAL_FILTERED_WMO_MODEL_SCOPE,
                )
            } else if version == 47 {
                (
                    GAMEOBJECT_EXPANDED_MATERIAL_FILTERED_WMO_GAMEOBJECT_SCOPE,
                    GAMEOBJECT_EXPANDED_MATERIAL_FILTERED_WMO_MODEL_SCOPE,
                )
            } else {
                (
                    SELECTED_GEOMETRY_GAMEOBJECT_SCOPE,
                    SELECTED_GEOMETRY_MODEL_SCOPE,
                )
            };
            if field(&mut lines, "gameobject-scope")? != gameobject_scope {
                return Err("product_plan_gameobject_scope");
            }
            if field(&mut lines, "model-scope")? != model_scope {
                return Err("product_plan_model_scope");
            }
            None
        } else if version == 29 {
            if field(&mut lines, "collision-profile")? != AVAILABLE_GEOMETRY_COLLISION_PROFILE {
                return Err("product_plan_collision_profile");
            }
            if field(&mut lines, "collision-domain")? != "all-available-maps" {
                return Err("product_plan_collision_domain");
            }
            if field(&mut lines, "collision-coverage")? != AVAILABLE_GEOMETRY_COVERAGE_PROFILE {
                return Err("product_plan_collision_coverage");
            }
            if field(&mut lines, "gameobject-scope")? != SELECTED_GEOMETRY_GAMEOBJECT_SCOPE {
                return Err("product_plan_gameobject_scope");
            }
            if field(&mut lines, "model-scope")? != SELECTED_GEOMETRY_MODEL_SCOPE {
                return Err("product_plan_model_scope");
            }
            None
        } else if matches!(version, 22 | 28) {
            if field(&mut lines, "collision-profile")? != SELECTED_GEOMETRY_COLLISION_PROFILE {
                return Err("product_plan_collision_profile");
            }
            let domain = decimal(
                field(&mut lines, "collision-domain")?
                    .strip_prefix("map:")
                    .ok_or("product_plan_collision_domain")?,
            )?;
            if field(&mut lines, "collision-coverage")? != SELECTED_GEOMETRY_COVERAGE_PROFILE {
                return Err("product_plan_collision_coverage");
            }
            if field(&mut lines, "gameobject-scope")? != SELECTED_GEOMETRY_GAMEOBJECT_SCOPE {
                return Err("product_plan_gameobject_scope");
            }
            if field(&mut lines, "model-scope")? != SELECTED_GEOMETRY_MODEL_SCOPE {
                return Err("product_plan_model_scope");
            }
            Some(domain)
        } else if matches!(version, 14 | 15 | 18) {
            let profile = if version == 14 {
                COMPATIBLE_COLLISION_VERIFY_PROFILE
            } else {
                COMPATIBLE_COLLISION_VERIFY_PROFILE_70450
            };
            if field(&mut lines, "collision-profile")? != profile {
                return Err("product_plan_collision_profile");
            }
            let domain = match field(&mut lines, "collision-domain")? {
                "all-available-maps" => None,
                text => Some(decimal(
                    text.strip_prefix("map:")
                        .ok_or("product_plan_collision_domain")?,
                )?),
            };
            if field(&mut lines, "collision-coverage")? != "compatible-collision-coverage-v1" {
                return Err("product_plan_collision_coverage");
            }
            domain
        } else if matches!(version, 8 | 21) {
            Some(map_domain::parse(&mut lines)?)
        } else {
            None
        };
        let mut components = 0;
        for name in field(&mut lines, "components")?.split(',') {
            let component = Component::parse(name).map_err(|_| "product_plan_components")?;
            if components & component.bit() != 0 {
                return Err("product_plan_components");
            }
            components |= component.bit();
        }
        let terrain_text = field(&mut lines, "terrain")?;
        let terrain = if matches!(version, 12 | 13 | 20 | 24 | 25 | 27 | 51)
            && terrain_text == "all-available-maps"
        {
            Selection::All
        } else {
            let selected = Selection::parse(terrain_text)?;
            if matches!(version, 12 | 13 | 20 | 24 | 25 | 27 | 51) && selected == Selection::All {
                return Err("product_plan_selection");
            }
            selected
        };
        let pack = boolean(field(&mut lines, "pack")?)?;
        let navigation = Selection::parse(field(&mut lines, "navigation")?)?;
        let big_base_unit = boolean(field(&mut lines, "big-base-unit")?)?;
        let skip_liquids = boolean(field(&mut lines, "skip-liquids")?)?;
        let maximum_slope = decimal(field(&mut lines, "maximum-slope")?)?;
        let non_steep_slope = decimal(field(&mut lines, "non-steep-slope")?)?;
        if maximum_slope > 90 || non_steep_slope > 90 {
            return Err("product_plan_slopes");
        }
        let offmesh = if version == 6
            || (version == 8 && navigation != Selection::None)
            || (version == 21 && navigation != Selection::None)
        {
            if field(&mut lines, "offmesh")? != OFFMESH_PROFILE {
                return Err("product_plan_offmesh");
            }
            let mut connections = Vec::new();
            while lines
                .clone()
                .next()
                .is_some_and(|line| line.starts_with("offmesh-connection\t"))
            {
                if connections.len() == MAX_OFFMESH_CONNECTIONS {
                    return Err("product_plan_offmesh_limit");
                }
                connections.push(OffMeshConnection::parse(
                    lines.next().ok_or("product_plan_format")?,
                )?);
            }
            Some(connections)
        } else {
            None
        };
        let mut db2 = Vec::new();
        for line in lines {
            if db2.len() == schema_catalog.max_entries() {
                return Err("product_plan_size");
            }
            let fields: Vec<_> = line.split('\t').collect();
            let ["db2", name, id] = fields.as_slice() else {
                return Err("product_plan_format");
            };
            db2.push(Entry {
                name: (*name).into(),
                id: decimal(id)?,
            });
        }
        let plan = Self {
            publication_profile: if version == 52 {
                PublicationProfile::Build70450AvailableNavigationV2
            } else if version == 51 {
                PublicationProfile::Build70450LiquidTerrainVmapCombinedV1
            } else if version == 50 {
                PublicationProfile::Build70450LiquidObjectCompleteSourceV1
            } else if version == 49 {
                PublicationProfile::Build70450GameObjectRootAndEncryptedUnavailableMaterialFilteredWmoGeometryV1
            } else if version == 48 {
                PublicationProfile::Build70450GameObjectMissingRootMaterialFilteredWmoGeometryV1
            } else if version == 47 {
                PublicationProfile::Build70450GameObjectExpandedMaterialFilteredWmoGeometryV1
            } else if version == 46 {
                PublicationProfile::Build70450AssemblyExpandedMaterialFilteredWmoGeometryV1
            } else if version == 45 {
                PublicationProfile::Build70450PlacementExpandedMaterialFilteredWmoGeometryV1
            } else if version == 44 {
                PublicationProfile::Build70450SourceExpandedV2MaterialFilteredWmoGeometryV1
            } else if version == 43 {
                PublicationProfile::Build70450ModelBytesExpandedMaterialFilteredWmoGeometryV1
            } else if version == 42 {
                PublicationProfile::Build70450ModelOutputExpandedMaterialFilteredWmoGeometryV1
            } else if version == 41 {
                PublicationProfile::Build70450SourceExpandedMaterialFilteredWmoGeometryV1
            } else if version == 40 {
                PublicationProfile::Build70450ModelHeavyMaterialFilteredWmoGeometryV1
            } else if version == 39 {
                PublicationProfile::Build70450ReadHeavyMaterialFilteredWmoGeometryV1
            } else if version == 17 {
                PublicationProfile::Build70450Db2ProvidedKeysV1
            } else if version == 18 {
                PublicationProfile::Build70450CollisionProvidedKeysV1
            } else if version == 19 {
                PublicationProfile::Build70450Db2GtCamerasV1
            } else if version == 20 {
                PublicationProfile::Build70450CompatibleTerrainV1
            } else if version == 21 {
                PublicationProfile::Build70450NavigationV1
            } else if version == 22 {
                PublicationProfile::Build70450SelectedGeometryV1
            } else if version == 23 {
                PublicationProfile::Build70450SelectedNavigationV1
            } else if version == 38 {
                PublicationProfile::Build70450SourceHeavyMaterialFilteredWmoGeometryV1
            } else if version == 37 {
                PublicationProfile::Build70450DependencyHeavyMaterialFilteredWmoGeometryV1
            } else if version == 36 {
                PublicationProfile::Build70450MaterialFilteredWmoGeometryV1
            } else if version == 35 {
                PublicationProfile::Build70450AvailableUnselectedWmoGeometryV1
            } else if version == 34 {
                PublicationProfile::Build70450AvailableEmptyWmoGeometryV1
            } else if version == 33 {
                PublicationProfile::Build70450AvailableVacatedWmoGeometryV1
            } else if version == 32 {
                PublicationProfile::Build70450AvailableModelRootAbsentRequestCapacityGeometryV1
            } else if version == 31 {
                PublicationProfile::Build70450AvailableModelRootAbsentGeometryV1
            } else if version == 30 {
                PublicationProfile::Build70450AvailableRootAbsentGeometryV1
            } else if version == 29 {
                PublicationProfile::Build70450AuthenticatedAvailableGeometryV1
            } else if version == 28 {
                PublicationProfile::Build70450AuthenticatedSelectedGeometryV1
            } else if version == 27 {
                PublicationProfile::Build70450PartialCatalogTerrainV1
            } else if version == 26 {
                PublicationProfile::Build70450Db2PartialV1
            } else if version == 25 {
                PublicationProfile::Build70450AuthenticatedTerrainPartialMapV1
            } else if version == 24 {
                PublicationProfile::Build70450AuthenticatedTerrainV1
            } else if version == 16 {
                PublicationProfile::Build70450Db2OnlyV1
            } else if version == 15 {
                PublicationProfile::Build70450CompatibleCollisionV1
            } else if version == 14 {
                PublicationProfile::Build70425CompatibleCollisionV1
            } else if version == 13 {
                PublicationProfile::Build70374CompatibleTerrainV1
            } else if version == 12 {
                PublicationProfile::Build70374AvailableTerrainV1
            } else if version == 9 {
                PublicationProfile::Build70338Db2GtCamerasV1
            } else if version == 11 {
                PublicationProfile::Build70374Db2GtCamerasV1
            } else if version == 10 {
                PublicationProfile::Build70374Db2OnlyV1
            } else {
                PublicationProfile::Legacy
            },
            build,
            schema_catalog,
            locale,
            index_layout,
            root_layout,
            decryption,
            collision_storage,
            map_domain,
            components,
            db2,
            terrain,
            navigation,
            pack,
            big_base_unit,
            skip_liquids,
            maximum_slope: maximum_slope as u8,
            non_steep_slope: non_steep_slope as u8,
            offmesh,
        };
        plan.validate()?;
        if plan.render() != input {
            return Err("product_plan_noncanonical");
        }
        Ok(plan)
    }

    /// Validate manually constructed plans before any source or output I/O.
    pub fn validate(&self) -> Result<(), &'static str> {
        if self.is_v52() {
            if self.navigation != Selection::All {
                return Err("product_plan_component_scope");
            }
            let mut selected = self.clone();
            selected.publication_profile = PublicationProfile::Build70450SelectedNavigationV1;
            selected.navigation = Selection::Map(0);
            return selected.validate();
        }
        self.publication_profile.validate(self)?;
        if !self.is_v14()
            && !self.is_v51()
            && !self.is_v15()
            && !self.is_v18()
            && !self.has_selected_geometry()
            && !self.is_v23()
            && !self.is_v24()
            && !self.is_v25()
            && !self.is_v26()
            && !self.is_v27()
            && !self.is_v29()
            && !self.is_v30()
            && !self.is_v31()
            && !self.is_v32()
            && !self.is_v33()
            && !self.is_v34()
            && !self.is_v35()
            && !self.is_v36()
            && !self.is_v37()
            && !self.is_v38()
            && !self.is_v39()
            && !self.is_v40()
            && !self.is_v41()
            && !self.is_v42()
            && !self.is_v43()
            && !self.is_v44()
            && !self.is_v45()
            && !self.is_v46()
            && !self.is_v47()
            && !self.is_v48()
            && !self.is_v49()
            && !self.is_v50()
            && !self.is_v51()
        {
            map_domain::validate(self)?;
        }
        if let Some(connections) = &self.offmesh {
            let legacy = self.schema_catalog == SchemaCatalog::PinnedMetadataV1
                && [70170, 70205].contains(&self.build);
            let build_70450 = self.is_v21()
                && self.build == 70450
                && self.schema_catalog == SchemaCatalog::WowDbDefs70450V1;
            if !(legacy || build_70450) || !self.contains(Component::Navigation) {
                return Err("product_plan_offmesh_scope");
            }
            if connections.len() > MAX_OFFMESH_CONNECTIONS {
                return Err("product_plan_offmesh_limit");
            }
            for connection in connections {
                connection.validate()?;
            }
        }
        if ![70170, 70205, 70338].contains(&self.build)
            && !self.is_v10()
            && !self.is_v11()
            && !self.is_available_terrain()
            && !self.is_v14()
            && !self.is_v15()
            && !self.is_v16()
            && !self.is_v17()
            && !self.is_v18()
            && !self.is_v19()
            && !self.is_v20()
            && !self.is_v21()
            && !self.has_selected_geometry()
            && !self.is_v23()
            && !self.is_v24()
            && !self.is_v25()
            && !self.is_v26()
            && !self.is_v27()
            && !self.is_v29()
            && !self.is_v30()
            && !self.is_v31()
            && !self.is_v32()
            && !self.is_v33()
            && !self.is_v34()
            && !self.is_v35()
            && !self.is_v36()
            && !self.is_v37()
            && !self.is_v38()
            && !self.is_v39()
            && !self.is_v40()
            && !self.is_v41()
            && !self.is_v42()
            && !self.is_v43()
            && !self.is_v44()
            && !self.is_v45()
            && !self.is_v46()
            && !self.is_v47()
            && !self.is_v48()
            && !self.is_v49()
            && !self.is_v50()
            && !self.is_v51()
            && !self.is_v53()
            && !self.is_v54()
            && !self.is_v55()
            && !self.is_v56()
            && !self.is_v57()
        {
            return Err("product_plan_build");
        }
        if self.schema_catalog == SchemaCatalog::WowDbDefs70205V1 && self.build != 70205 {
            return Err("product_plan_schema_catalog");
        }
        if (self.schema_catalog == SchemaCatalog::WowDbDefs70338V1 && self.build != 70338)
            || (self.build == 70338 && self.schema_catalog != SchemaCatalog::WowDbDefs70338V1)
        {
            return Err("product_plan_schema_catalog");
        }
        if (self.schema_catalog == SchemaCatalog::WowDbDefs70374V1 || self.build == 70374)
            && !self.is_v10()
            && !self.is_v11()
            && !self.is_available_terrain()
        {
            return Err("product_plan_schema_catalog");
        }
        if (self.schema_catalog == SchemaCatalog::WowDbDefs70425V1 || self.build == 70425)
            && !self.is_v14()
        {
            return Err("product_plan_schema_catalog");
        }
        if (self.schema_catalog == SchemaCatalog::WowDbDefs70450V1 || self.build == 70450)
            && !self.is_v15()
            && !self.is_v16()
            && !self.is_v17()
            && !self.is_v18()
            && !self.is_v19()
            && !self.is_v20()
            && !self.is_v21()
            && !self.has_selected_geometry()
            && !self.is_v23()
            && !self.is_v24()
            && !self.is_v25()
            && !self.is_v26()
            && !self.is_v27()
            && !self.is_v29()
            && !self.is_v30()
            && !self.is_v31()
            && !self.is_v32()
            && !self.is_v33()
            && !self.is_v34()
            && !self.is_v35()
            && !self.is_v36()
            && !self.is_v37()
            && !self.is_v38()
            && !self.is_v39()
            && !self.is_v40()
            && !self.is_v41()
            && !self.is_v42()
            && !self.is_v43()
            && !self.is_v44()
            && !self.is_v45()
            && !self.is_v46()
            && !self.is_v47()
            && !self.is_v48()
            && !self.is_v49()
            && !self.is_v50()
            && !self.is_v51()
            && !self.is_v53()
            && !self.is_v54()
            && !self.is_v55()
            && !self.is_v56()
            && !self.is_v57()
        {
            return Err("product_plan_schema_catalog");
        }
        if self.publication_profile == PublicationProfile::Legacy
            && self.build == 70338
            && self.components != Component::Db2.bit()
        {
            return Err("product_plan_component_scope");
        }
        if self.components == 0 || self.components & !63 != 0 {
            return Err("product_plan_components");
        }
        if self.is_v51() && self.components != 25 {
            return Err("product_plan_component_scope");
        }
        if let CollisionStorage::LeasedDiskV1(budget) = self.collision_storage {
            if !self.contains(Component::Collision) {
                return Err("product_plan_dependencies");
            }
            budget.validate()?;
        }
        if self.db2.len() > self.schema_catalog.max_entries() {
            return Err("product_plan_size");
        }
        let mut ids = BTreeSet::new();
        let mut previous: Option<&str> = None;
        for entry in &self.db2 {
            let metadata = match self.schema_catalog {
                SchemaCatalog::PinnedMetadataV1 => {
                    catalog::find(&entry.name).ok_or("product_plan_schema")?
                }
                SchemaCatalog::WowDbDefs70205V1 => {
                    catalog::resolve_for(self.schema_catalog.id(), &entry.name)
                        .map_err(|_| "product_plan_schema")?
                        .ok_or("product_plan_schema")?
                        .metadata()
                }
                SchemaCatalog::WowDbDefs70338V1
                | SchemaCatalog::WowDbDefs70374V1
                | SchemaCatalog::WowDbDefs70425V1
                | SchemaCatalog::WowDbDefs70450V1 => {
                    catalog::resolve_for(self.schema_catalog.id(), &entry.name)
                        .map_err(|_| "product_plan_schema")?
                        .ok_or("product_plan_schema")?
                        .metadata()
                }
            };
            let ordered = if self.is_v51() {
                match entry.name.as_str() {
                    "GameObjectDisplayInfo" => previous.is_none(),
                    "Map" => previous == Some("GameObjectDisplayInfo"),
                    "LiquidObject" => previous == Some("Map"),
                    "LiquidMaterial" => previous == Some("LiquidObject"),
                    "LiquidType" => previous == Some("LiquidMaterial"),
                    _ => false,
                }
            } else if self.is_v50() {
                match entry.name.as_str() {
                    "GameObjectDisplayInfo" => previous.is_none(),
                    "Map" => previous == Some("GameObjectDisplayInfo"),
                    "LiquidObject" => previous == Some("Map"),
                    _ => false,
                }
            } else {
                previous.is_none_or(|name| name < entry.name.as_str())
            };
            if metadata.file_data_id != entry.id || !ids.insert(entry.id) || !ordered {
                return Err("product_plan_schema");
            }
            previous = Some(&entry.name);
        }
        if self.contains(Component::Db2) == self.db2.is_empty() {
            return Err("product_plan_dependencies");
        }
        self.terrain.validate()?;
        self.navigation.validate()?;
        if self.contains(Component::Terrain) == (self.terrain == Selection::None)
            || self.contains(Component::Navigation) == (self.navigation == Selection::None)
        {
            return Err("product_plan_dependencies");
        }
        if self.maximum_slope > 90 || self.non_steep_slope > self.maximum_slope {
            return Err("product_plan_slopes");
        }
        let has_db2 = |name: &str| self.db2.iter().any(|entry| entry.name == name);
        if self.contains(Component::Cameras)
            && (!self.contains(Component::Db2) || !has_db2("CinematicCamera"))
        {
            return Err("product_plan_dependencies");
        }
        if self.contains(Component::Navigation)
            && !self.is_v23()
            && !self.is_v53()
            && !self.is_v54()
            && !self.is_v55()
            && !self.is_v56()
            && !self.is_v57()
        {
            if !self.contains(Component::Terrain)
                || !self.contains(Component::Collision)
                || !self.contains(Component::Db2)
                || (self.map_domain.is_none() && self.terrain != Selection::All)
                || !has_db2("Map")
                || !has_db2("LiquidType")
            {
                return Err("product_plan_dependencies");
            }
        } else if self.big_base_unit
            || self.skip_liquids
            || self.maximum_slope != 55
            || self.non_steep_slope != 55
        {
            return Err("product_plan_options");
        }
        if self.try_render()?.len() > MAX_PLAN_BYTES {
            return Err("product_plan_size");
        }
        Ok(())
    }

    pub const fn contains(&self, component: Component) -> bool {
        self.components & component.bit() != 0
    }

    pub const fn version(&self) -> u8 {
        if self.is_v57() {
            return 57;
        }
        if self.is_v56() {
            return 56;
        }
        if self.is_v55() {
            return 55;
        }
        if self.is_v54() {
            return 54;
        }
        if self.is_v53() {
            return 53;
        }
        if self.is_v52() {
            return 52;
        }
        if self.is_v51() {
            return 51;
        }
        if self.is_v50() {
            return 50;
        }
        if self.is_v49() {
            return 49;
        }
        if self.is_v48() {
            return 48;
        }
        if self.is_v47() {
            return 47;
        }
        if self.is_v46() {
            return 46;
        }
        if self.is_v45() {
            return 45;
        }
        if self.is_v44() {
            return 44;
        }
        if self.is_v43() {
            return 43;
        }
        if self.is_v42() {
            return 42;
        }
        if self.is_v41() {
            return 41;
        }
        if self.is_v40() {
            return 40;
        }
        if self.is_v39() {
            return 39;
        }
        if self.is_v38() {
            return 38;
        }
        if self.is_v37() {
            return 37;
        }
        if self.is_v36() {
            return 36;
        }
        if self.is_v35() {
            return 35;
        }
        if self.is_v34() {
            return 34;
        }
        if self.is_v33() {
            return 33;
        }
        if self.is_v32() {
            return 32;
        }
        if self.is_v31() {
            return 31;
        }
        if self.is_v30() {
            return 30;
        }
        if self.is_v29() {
            return 29;
        }
        if self.is_v28() {
            return 28;
        }
        if self.is_v27() {
            return 27;
        }
        if self.is_v26() {
            return 26;
        }
        if self.is_v25() {
            return 25;
        }
        if self.is_v24() {
            return 24;
        }
        if self.is_v23() {
            return 23;
        }
        if self.is_v22() {
            return 22;
        }
        if self.is_v21() {
            return 21;
        }
        if self.is_v20() {
            return 20;
        }
        if self.is_v19() {
            return 19;
        }
        if self.is_v18() {
            return 18;
        }
        if self.is_v17() {
            return 17;
        }
        if self.is_v16() {
            return 16;
        }
        if self.is_v15() {
            return 15;
        }
        if self.is_v14() {
            return 14;
        }
        if self.is_v13() {
            return 13;
        }
        if self.is_v12() {
            return 12;
        }
        if self.is_v11() {
            return 11;
        }
        if self.is_v10() {
            return 10;
        }
        if self.is_v9() {
            return 9;
        }
        if self.map_domain.is_some() {
            return 8;
        }
        if self.offmesh.is_some() {
            return 6;
        }
        match self.schema_catalog {
            SchemaCatalog::WowDbDefs70205V1 => return 4,
            SchemaCatalog::WowDbDefs70338V1 => return 5,
            SchemaCatalog::WowDbDefs70374V1
            | SchemaCatalog::WowDbDefs70425V1
            | SchemaCatalog::WowDbDefs70450V1 => {}
            SchemaCatalog::PinnedMetadataV1 => {}
        }
        if matches!(self.collision_storage, CollisionStorage::LeasedDiskV1(_)) {
            return 3;
        }
        match self.decryption {
            Decryption::Disabled => 1,
            Decryption::ProvidedSalsa20V1 => 2,
        }
    }

    pub const fn is_v52(&self) -> bool {
        matches!(
            self.publication_profile,
            PublicationProfile::Build70450AvailableNavigationV1
        )
    }

    pub const fn is_v53(&self) -> bool {
        matches!(
            self.publication_profile,
            PublicationProfile::Build70450AvailableNavigationV2
        )
    }

    pub const fn is_v54(&self) -> bool {
        matches!(
            self.publication_profile,
            PublicationProfile::Build70450AvailableNavigationV3
        )
    }

    pub const fn is_v55(&self) -> bool {
        matches!(
            self.publication_profile,
            PublicationProfile::Build70450AvailableNavigationV4
        )
    }

    pub const fn is_v56(&self) -> bool {
        matches!(
            self.publication_profile,
            PublicationProfile::Build70450AvailableNavigationV5
        )
    }

    pub const fn is_v57(&self) -> bool {
        matches!(
            self.publication_profile,
            PublicationProfile::Build70450AvailableNavigationV6
        )
    }

    pub const fn contract(&self) -> &'static str {
        if self.is_v57() {
            return "forever-product-v57";
        }
        if self.is_v56() {
            return "forever-product-v56";
        }
        if self.is_v55() {
            return "forever-product-v55";
        }
        if self.is_v54() {
            return "forever-product-v54";
        }
        if self.is_v53() {
            return "forever-product-v53";
        }
        if self.is_v52() {
            return "forever-product-v52";
        }
        if self.is_v51() {
            return "forever-product-v51";
        }
        if self.is_v50() {
            return "forever-product-v50";
        }
        if self.is_v49() {
            return "forever-product-v49";
        }
        if self.is_v48() {
            return "forever-product-v48";
        }
        if self.is_v47() {
            return "forever-product-v47";
        }
        if self.is_v46() {
            return "forever-product-v46";
        }
        if self.is_v45() {
            return "forever-product-v45";
        }
        if self.is_v44() {
            return "forever-product-v44";
        }
        if self.is_v43() {
            return "forever-product-v43";
        }
        if self.is_v42() {
            return "forever-product-v42";
        }
        if self.is_v41() {
            return "forever-product-v41";
        }
        if self.is_v40() {
            return "forever-product-v40";
        }
        if self.is_v39() {
            return "forever-product-v39";
        }
        if self.is_v38() {
            return "forever-product-v38";
        }
        if self.is_v37() {
            return "forever-product-v37";
        }
        if self.is_v36() {
            return "forever-product-v36";
        }
        if self.is_v35() {
            return "forever-product-v35";
        }
        if self.is_v34() {
            return "forever-product-v34";
        }
        if self.is_v33() {
            return "forever-product-v33";
        }
        if self.is_v32() {
            return "forever-product-v32";
        }
        if self.is_v31() {
            return "forever-product-v31";
        }
        if self.is_v30() {
            return "forever-product-v30";
        }
        if self.is_v29() {
            return "forever-product-v29";
        }
        if self.is_v28() {
            return "forever-product-v28";
        }
        if self.is_v27() {
            return "forever-product-v27";
        }
        if self.is_v26() {
            return "forever-product-v26";
        }
        if self.is_v25() {
            return "forever-product-v25";
        }
        if self.is_v24() {
            return "forever-product-v24";
        }
        if self.is_v23() {
            return "forever-product-v23";
        }
        if self.is_v22() {
            return "forever-product-v22";
        }
        if self.is_v21() {
            return "forever-product-v21";
        }
        if self.is_v20() {
            return "forever-product-v20";
        }
        if self.is_v19() {
            return "forever-product-v19";
        }
        if self.is_v18() {
            return "forever-product-v18";
        }
        if self.is_v17() {
            return "forever-product-v17";
        }
        if self.is_v16() {
            return "forever-product-v16";
        }
        if self.is_v15() {
            return "forever-product-v15";
        }
        if self.is_v14() {
            return "forever-product-v14";
        }
        if self.is_v13() {
            return "forever-product-v13";
        }
        if self.is_v12() {
            return "forever-product-v12";
        }
        if self.is_v11() {
            return "forever-product-v11";
        }
        if self.is_v10() {
            return "forever-product-v10";
        }
        if self.is_v9() {
            return "forever-product-v9";
        }
        if self.map_domain.is_some() {
            return "forever-product-v8";
        }
        if self.offmesh.is_some() {
            return "forever-product-v6";
        }
        match self.schema_catalog {
            SchemaCatalog::WowDbDefs70205V1 => return "forever-product-v4",
            SchemaCatalog::WowDbDefs70338V1 => return "forever-product-v5",
            SchemaCatalog::WowDbDefs70374V1
            | SchemaCatalog::WowDbDefs70425V1
            | SchemaCatalog::WowDbDefs70450V1 => {}
            SchemaCatalog::PinnedMetadataV1 => {}
        }
        if matches!(self.collision_storage, CollisionStorage::LeasedDiskV1(_)) {
            return "forever-product-v3";
        }
        match self.decryption {
            Decryption::Disabled => "forever-product-v1",
            Decryption::ProvidedSalsa20V1 => "forever-product-v2",
        }
    }

    pub const fn is_v9(&self) -> bool {
        matches!(
            self.publication_profile,
            PublicationProfile::Build70338Db2GtCamerasV1
        )
    }

    pub const fn is_v10(&self) -> bool {
        matches!(
            self.publication_profile,
            PublicationProfile::Build70374Db2OnlyV1
        )
    }

    pub const fn is_v12(&self) -> bool {
        matches!(
            self.publication_profile,
            PublicationProfile::Build70374AvailableTerrainV1
        )
    }

    /// Explicit Linux70425 disabled-key compatible GNU15-v2 collision publication.
    pub const fn is_v14(&self) -> bool {
        matches!(
            self.publication_profile,
            PublicationProfile::Build70425CompatibleCollisionV1
        )
    }

    /// Explicit Linux70450 disabled-key compatible GNU15-v2 collision publication.
    pub const fn is_v15(&self) -> bool {
        matches!(
            self.publication_profile,
            PublicationProfile::Build70450CompatibleCollisionV1
        )
    }

    /// Explicit Linux build70450 disabled-key DB2-only publication.
    pub const fn is_v16(&self) -> bool {
        matches!(
            self.publication_profile,
            PublicationProfile::Build70450Db2OnlyV1
        )
    }

    /// Explicit Linux build70450 DB2-only publication with supplied keys.
    pub const fn is_v17(&self) -> bool {
        matches!(
            self.publication_profile,
            PublicationProfile::Build70450Db2ProvidedKeysV1
        )
    }

    /// Explicit Linux build70450 compatible GNU15-v2 collision with supplied keys.
    pub const fn is_v18(&self) -> bool {
        matches!(
            self.publication_profile,
            PublicationProfile::Build70450CollisionProvidedKeysV1
        )
    }

    /// Explicit Linux build70450 DB2+GT+cameras publication.
    pub const fn is_v19(&self) -> bool {
        matches!(
            self.publication_profile,
            PublicationProfile::Build70450Db2GtCamerasV1
        )
    }

    /// Explicit Linux build70450 compatible available-Map terrain publication.
    pub const fn is_v20(&self) -> bool {
        matches!(
            self.publication_profile,
            PublicationProfile::Build70450CompatibleTerrainV1
        )
    }

    /// Explicit Linux build70450 compatible map-domain navigation publication.
    pub const fn is_v21(&self) -> bool {
        matches!(
            self.publication_profile,
            PublicationProfile::Build70450NavigationV1
        )
    }

    /// Explicit Linux build-70450 selected-map collision-geometry product.
    pub const fn is_v22(&self) -> bool {
        matches!(
            self.publication_profile,
            PublicationProfile::Build70450SelectedGeometryV1
        )
    }

    /// Explicit Linux build-70450 selected navigation over sealed inputs.
    pub const fn is_v23(&self) -> bool {
        matches!(
            self.publication_profile,
            PublicationProfile::Build70450SelectedNavigationV1
        )
    }

    /// Explicit Linux build-70450 authenticated Map+terrain product.
    pub const fn is_v24(&self) -> bool {
        matches!(
            self.publication_profile,
            PublicationProfile::Build70450AuthenticatedTerrainV1
        )
    }

    /// Explicit Linux build-70450 authenticated terrain with declared partial Map sections.
    pub const fn is_v25(&self) -> bool {
        matches!(
            self.publication_profile,
            PublicationProfile::Build70450AuthenticatedTerrainPartialMapV1
        )
    }

    /// Explicit Linux build-70450 DB2-only publication with declared skipped sections.
    pub const fn is_v26(&self) -> bool {
        matches!(
            self.publication_profile,
            PublicationProfile::Build70450Db2PartialV1
        )
    }

    /// Explicit Linux build-70450 DB2 catalog and authenticated terrain with
    /// exclusively per-table declarations for skipped sections.
    pub const fn is_v27(&self) -> bool {
        matches!(
            self.publication_profile,
            PublicationProfile::Build70450PartialCatalogTerrainV1
        )
    }

    /// Explicit Linux build-70450 selected VMap geometry with independently
    /// derived Map coverage and per-table partial declarations.
    pub const fn is_v28(&self) -> bool {
        matches!(
            self.publication_profile,
            PublicationProfile::Build70450AuthenticatedSelectedGeometryV1
        )
    }

    /// Explicit Linux build-70450 available VMap geometry with independently
    /// derived coverage over every admitted active row.
    pub const fn is_v29(&self) -> bool {
        matches!(
            self.publication_profile,
            PublicationProfile::Build70450AuthenticatedAvailableGeometryV1
        )
    }

    /// Explicit Linux build-70450 available VMap geometry with declared
    /// root-absent WDT omissions and independently derived Map coverage.
    pub const fn is_v30(&self) -> bool {
        matches!(
            self.publication_profile,
            PublicationProfile::Build70450AvailableRootAbsentGeometryV1
        )
    }

    /// Explicit Linux build-70450 available VMap geometry with declared
    /// root-absent WDT and WMO-doodad M2 omissions.
    pub const fn is_v31(&self) -> bool {
        matches!(
            self.publication_profile,
            PublicationProfile::Build70450AvailableModelRootAbsentGeometryV1
        )
    }

    /// Explicit Linux build-70450 v31 geometry with bounded resident request capacity.
    pub const fn is_v32(&self) -> bool {
        matches!(
            self.publication_profile,
            PublicationProfile::Build70450AvailableModelRootAbsentRequestCapacityGeometryV1
        )
    }

    /// Explicit Linux build-70450 available geometry with vacated-WMO declarations.
    pub const fn is_v33(&self) -> bool {
        matches!(
            self.publication_profile,
            PublicationProfile::Build70450AvailableVacatedWmoGeometryV1
        )
    }

    /// Explicit Linux build-70450 available geometry with separate vacated-WMO
    /// and intrinsically empty-WMO declarations.
    pub const fn is_v34(&self) -> bool {
        matches!(
            self.publication_profile,
            PublicationProfile::Build70450AvailableEmptyWmoGeometryV1
        )
    }

    /// Explicit Linux build-70450 available geometry with separate declarations
    /// for WMO whose source geometry is not used by collision selection.
    pub const fn is_v35(&self) -> bool {
        matches!(
            self.publication_profile,
            PublicationProfile::Build70450AvailableUnselectedWmoGeometryV1
        )
    }

    /// Explicit Linux build-70450 available geometry with material-filtered WMO declarations.
    pub const fn is_v36(&self) -> bool {
        matches!(
            self.publication_profile,
            PublicationProfile::Build70450MaterialFilteredWmoGeometryV1
        )
    }

    pub const fn is_v37(&self) -> bool {
        matches!(
            self.publication_profile,
            PublicationProfile::Build70450DependencyHeavyMaterialFilteredWmoGeometryV1
        )
    }

    pub const fn is_v38(&self) -> bool {
        matches!(
            self.publication_profile,
            PublicationProfile::Build70450SourceHeavyMaterialFilteredWmoGeometryV1
        )
    }

    pub const fn is_v39(&self) -> bool {
        matches!(
            self.publication_profile,
            PublicationProfile::Build70450ReadHeavyMaterialFilteredWmoGeometryV1
        )
    }

    pub const fn is_v40(&self) -> bool {
        matches!(
            self.publication_profile,
            PublicationProfile::Build70450ModelHeavyMaterialFilteredWmoGeometryV1
        )
    }

    pub const fn is_v41(&self) -> bool {
        matches!(
            self.publication_profile,
            PublicationProfile::Build70450SourceExpandedMaterialFilteredWmoGeometryV1
        )
    }

    pub const fn is_v42(&self) -> bool {
        matches!(
            self.publication_profile,
            PublicationProfile::Build70450ModelOutputExpandedMaterialFilteredWmoGeometryV1
        )
    }

    pub const fn is_v43(&self) -> bool {
        matches!(
            self.publication_profile,
            PublicationProfile::Build70450ModelBytesExpandedMaterialFilteredWmoGeometryV1
        )
    }

    pub const fn is_v44(&self) -> bool {
        matches!(
            self.publication_profile,
            PublicationProfile::Build70450SourceExpandedV2MaterialFilteredWmoGeometryV1
        )
    }

    pub const fn is_v45(&self) -> bool {
        matches!(
            self.publication_profile,
            PublicationProfile::Build70450PlacementExpandedMaterialFilteredWmoGeometryV1
        )
    }

    pub const fn is_v46(&self) -> bool {
        matches!(
            self.publication_profile,
            PublicationProfile::Build70450AssemblyExpandedMaterialFilteredWmoGeometryV1
        )
    }

    pub const fn is_v47(&self) -> bool {
        matches!(
            self.publication_profile,
            PublicationProfile::Build70450GameObjectExpandedMaterialFilteredWmoGeometryV1
        )
    }

    pub const fn is_v48(&self) -> bool {
        matches!(
            self.publication_profile,
            PublicationProfile::Build70450GameObjectMissingRootMaterialFilteredWmoGeometryV1
        )
    }

    pub const fn is_v49(&self) -> bool {
        matches!(
            self.publication_profile,
            PublicationProfile::Build70450GameObjectRootAndEncryptedUnavailableMaterialFilteredWmoGeometryV1
        )
    }

    pub const fn is_v50(&self) -> bool {
        matches!(
            self.publication_profile,
            PublicationProfile::Build70450LiquidObjectCompleteSourceV1
        )
    }

    pub const fn is_v51(&self) -> bool {
        matches!(
            self.publication_profile,
            PublicationProfile::Build70450LiquidTerrainVmapCombinedV1
        )
    }

    pub const fn has_selected_geometry(&self) -> bool {
        self.is_v22() || self.is_v28()
    }

    pub const fn is_v13(&self) -> bool {
        matches!(
            self.publication_profile,
            PublicationProfile::Build70374CompatibleTerrainV1
        )
    }

    pub const fn has_authenticated_map_terrain(&self) -> bool {
        self.is_v24() || self.is_v25() || self.is_v27() || self.is_v51()
    }

    pub const fn is_available_terrain(&self) -> bool {
        self.is_v12() || self.is_v13() || self.is_v20() || self.has_authenticated_map_terrain()
    }

    pub const fn is_any_available_terrain(&self) -> bool {
        self.is_available_terrain() || self.is_v20() || self.is_v24() || self.is_v51()
    }

    pub const fn is_compatible_terrain(&self) -> bool {
        self.is_v13() || self.is_v20()
    }

    pub const fn is_v11(&self) -> bool {
        matches!(
            self.publication_profile,
            PublicationProfile::Build70374Db2GtCamerasV1
        )
    }

    pub const fn is_gt_cameras(&self) -> bool {
        self.is_v9() || self.is_v11() || self.is_v19()
    }

    pub const fn requires_keys(&self) -> bool {
        matches!(self.decryption, Decryption::ProvidedSalsa20V1)
    }

    /// Compatibility renderer for already-validated plans.
    ///
    /// Use `try_render` when a plan may have been constructed or modified
    /// directly; this helper returns an empty string rather than panicking if
    /// rendering cannot satisfy the v22 map-domain invariant.
    pub fn render(&self) -> String {
        self.try_render().unwrap_or_default()
    }

    /// Emit a canonical plan or return a typed plan error for an incomplete
    /// manually-constructed profile.
    pub fn try_render(&self) -> Result<String, &'static str> {
        if self.is_v57() {
            if self.navigation != Selection::All {
                return Err("product_plan_selection");
            }
            let mut selected = self.clone();
            selected.publication_profile = PublicationProfile::Build70450SelectedNavigationV1;
            selected.navigation = Selection::Map(0);
            return Ok(selected
                .try_render()?
                .replacen("forever-product\t23\n", "forever-product\t57\n", 1)
                .replacen("navigation\tmap:0\n", "navigation\tall-available-maps\n", 1));
        }
        if self.is_v56() {
            if self.navigation != Selection::All {
                return Err("product_plan_selection");
            }
            let mut selected = self.clone();
            selected.publication_profile = PublicationProfile::Build70450SelectedNavigationV1;
            selected.navigation = Selection::Map(0);
            return Ok(selected
                .try_render()?
                .replacen("forever-product\t23\n", "forever-product\t56\n", 1)
                .replacen("navigation\tmap:0\n", "navigation\tall-available-maps\n", 1));
        }
        if self.is_v55() {
            if self.navigation != Selection::All {
                return Err("product_plan_selection");
            }
            let mut selected = self.clone();
            selected.publication_profile = PublicationProfile::Build70450SelectedNavigationV1;
            selected.navigation = Selection::Map(0);
            return Ok(selected
                .try_render()?
                .replacen("forever-product\t23\n", "forever-product\t55\n", 1)
                .replacen("navigation\tmap:0\n", "navigation\tall-available-maps\n", 1));
        }
        if self.is_v54() {
            if self.navigation != Selection::All {
                return Err("product_plan_selection");
            }
            let mut selected = self.clone();
            selected.publication_profile = PublicationProfile::Build70450SelectedNavigationV1;
            selected.navigation = Selection::Map(0);
            return Ok(selected
                .try_render()?
                .replacen("forever-product\t23\n", "forever-product\t54\n", 1)
                .replacen("navigation\tmap:0\n", "navigation\tall-available-maps\n", 1));
        }
        if self.is_v53() {
            if self.navigation != Selection::All {
                return Err("product_plan_selection");
            }
            let mut selected = self.clone();
            selected.publication_profile = PublicationProfile::Build70450SelectedNavigationV1;
            selected.navigation = Selection::Map(0);
            return Ok(selected
                .try_render()?
                .replacen("forever-product\t23\n", "forever-product\t53\n", 1)
                .replacen("navigation\tmap:0\n", "navigation\tall-available-maps\n", 1));
        }
        if self.is_v52() {
            if self.navigation != Selection::All {
                return Err("product_plan_selection");
            }
            let mut selected = self.clone();
            selected.publication_profile = PublicationProfile::Build70450SelectedNavigationV1;
            selected.navigation = Selection::Map(0);
            return Ok(selected
                .try_render()?
                .replacen("forever-product\t23\n", "forever-product\t52\n", 1)
                .replacen("navigation\tmap:0\n", "navigation\tall-available-maps\n", 1));
        }
        if self.publication_profile == PublicationProfile::Legacy
            && matches!(self.version(), 48..=51)
        {
            return Err("product_plan_publication_profile");
        }
        let index = match self.index_layout {
            local_index::Layout::Contiguous => "contiguous",
            local_index::Layout::Paged => "paged",
        };
        let root = match self.root_layout {
            root::Layout::Headerless18125 => "headerless-18125",
            root::Layout::Header30080 => "header-30080",
            root::Layout::Header50893V1 => "header-50893-v1",
            root::Layout::Header50893V2 => "header-50893-v2",
        };
        let components = Component::ALL
            .into_iter()
            .filter(|component| self.contains(*component))
            .map(Component::name)
            .collect::<Vec<_>>()
            .join(",");
        let schema_catalog = match self.schema_catalog {
            SchemaCatalog::PinnedMetadataV1 => "",
            SchemaCatalog::WowDbDefs70205V1 => "schema-catalog\twowdbdefs-70205-v1\n",
            SchemaCatalog::WowDbDefs70338V1 => "schema-catalog\twowdbdefs-70338-v1\n",
            SchemaCatalog::WowDbDefs70374V1 => "schema-catalog\twowdbdefs-70374-v1\n",
            SchemaCatalog::WowDbDefs70425V1 => "schema-catalog\twowdbdefs-70425-v1\n",
            SchemaCatalog::WowDbDefs70450V1 => "schema-catalog\twowdbdefs-70450-v1\n",
        };
        let decryption = match (self.version(), self.decryption) {
            (3..=6 | 8..=51, Decryption::Disabled) => "decryption\tdisabled\n",
            (_, Decryption::Disabled) => "",
            (_, Decryption::ProvidedSalsa20V1) => "decryption\tprovided-salsa20-v1\n",
        };
        let partial_encrypted_sections = if matches!(self.version(), 25..=51) {
            "partial_encrypted_sections\tdeclared-skip\n"
        } else {
            ""
        };
        let collision_storage = self.collision_storage.render();
        let map_domain = if self.is_v51() {
            format!(
                "collision-profile\t{DB2_TERRAIN_LIQUID_VMAP_PRODUCT_PROFILE}\ncollision-domain\tall-available-maps\ncollision-coverage\t{DB2_TERRAIN_LIQUID_VMAP_COVERAGE_PROFILE}\ngameobject-scope\t{GAMEOBJECT_ROOT_ENCRYPTED_UNAVAILABLE_MATERIAL_FILTERED_WMO_GAMEOBJECT_SCOPE}\nmodel-scope\t{GAMEOBJECT_ROOT_ENCRYPTED_UNAVAILABLE_MATERIAL_FILTERED_WMO_MODEL_SCOPE}\n"
            )
        } else if self.is_v50() {
            format!(
                "collision-profile\t{LIQUID_OBJECT_COMPLETE_SOURCE_PROFILE}\ncollision-domain\tall-available-maps\ncollision-coverage\t{LIQUID_OBJECT_COMPLETE_SOURCE_COVERAGE_PROFILE}\ngameobject-scope\tsource-only-v1\nmodel-scope\tsource-only-v1\n"
            )
        } else if self.is_v49() {
            format!(
                "collision-profile\t{GAMEOBJECT_ROOT_ENCRYPTED_UNAVAILABLE_MATERIAL_FILTERED_WMO_COLLISION_PROFILE}\ncollision-domain\tall-available-maps\ncollision-coverage\t{GAMEOBJECT_ROOT_ENCRYPTED_UNAVAILABLE_MATERIAL_FILTERED_WMO_COVERAGE_PROFILE}\ngameobject-scope\t{GAMEOBJECT_ROOT_ENCRYPTED_UNAVAILABLE_MATERIAL_FILTERED_WMO_GAMEOBJECT_SCOPE}\nmodel-scope\t{GAMEOBJECT_ROOT_ENCRYPTED_UNAVAILABLE_MATERIAL_FILTERED_WMO_MODEL_SCOPE}\n"
            )
        } else if self.is_v48() {
            format!(
                "collision-profile\t{GAMEOBJECT_MISSING_ROOT_MATERIAL_FILTERED_WMO_COLLISION_PROFILE}\ncollision-domain\tall-available-maps\ncollision-coverage\t{GAMEOBJECT_MISSING_ROOT_MATERIAL_FILTERED_WMO_COVERAGE_PROFILE}\ngameobject-scope\t{GAMEOBJECT_MISSING_ROOT_MATERIAL_FILTERED_WMO_GAMEOBJECT_SCOPE}\nmodel-scope\t{GAMEOBJECT_MISSING_ROOT_MATERIAL_FILTERED_WMO_MODEL_SCOPE}\n"
            )
        } else if self.is_v47() {
            format!(
                "collision-profile\t{GAMEOBJECT_EXPANDED_MATERIAL_FILTERED_WMO_COLLISION_PROFILE}\ncollision-domain\tall-available-maps\ncollision-coverage\t{GAMEOBJECT_EXPANDED_MATERIAL_FILTERED_WMO_COVERAGE_PROFILE}\ngameobject-scope\t{GAMEOBJECT_EXPANDED_MATERIAL_FILTERED_WMO_GAMEOBJECT_SCOPE}\nmodel-scope\t{GAMEOBJECT_EXPANDED_MATERIAL_FILTERED_WMO_MODEL_SCOPE}\n"
            )
        } else if self.is_v46() {
            format!(
                "collision-profile\t{ASSEMBLY_EXPANDED_MATERIAL_FILTERED_WMO_COLLISION_PROFILE}\ncollision-domain\tall-available-maps\ncollision-coverage\t{ASSEMBLY_EXPANDED_MATERIAL_FILTERED_WMO_COVERAGE_PROFILE}\ngameobject-scope\t{SELECTED_GEOMETRY_GAMEOBJECT_SCOPE}\nmodel-scope\t{SELECTED_GEOMETRY_MODEL_SCOPE}\n"
            )
        } else if self.is_v45() {
            format!(
                "collision-profile\t{PLACEMENT_EXPANDED_MATERIAL_FILTERED_WMO_COLLISION_PROFILE}\ncollision-domain\tall-available-maps\ncollision-coverage\t{PLACEMENT_EXPANDED_MATERIAL_FILTERED_WMO_COVERAGE_PROFILE}\ngameobject-scope\t{SELECTED_GEOMETRY_GAMEOBJECT_SCOPE}\nmodel-scope\t{SELECTED_GEOMETRY_MODEL_SCOPE}\n"
            )
        } else if self.is_v44() {
            format!(
                "collision-profile\t{SOURCE_EXPANDED_V2_MATERIAL_FILTERED_WMO_COLLISION_PROFILE}\ncollision-domain\tall-available-maps\ncollision-coverage\t{SOURCE_EXPANDED_V2_MATERIAL_FILTERED_WMO_COVERAGE_PROFILE}\ngameobject-scope\t{SELECTED_GEOMETRY_GAMEOBJECT_SCOPE}\nmodel-scope\t{SELECTED_GEOMETRY_MODEL_SCOPE}\n"
            )
        } else if self.is_v43() {
            format!(
                "collision-profile\t{MODEL_BYTES_EXPANDED_MATERIAL_FILTERED_WMO_COLLISION_PROFILE}\ncollision-domain\tall-available-maps\ncollision-coverage\t{MODEL_BYTES_EXPANDED_MATERIAL_FILTERED_WMO_COVERAGE_PROFILE}\ngameobject-scope\t{SELECTED_GEOMETRY_GAMEOBJECT_SCOPE}\nmodel-scope\t{SELECTED_GEOMETRY_MODEL_SCOPE}\n"
            )
        } else if self.is_v42() {
            format!(
                "collision-profile\t{MODEL_OUTPUT_EXPANDED_MATERIAL_FILTERED_WMO_COLLISION_PROFILE}\ncollision-domain\tall-available-maps\ncollision-coverage\t{MODEL_OUTPUT_EXPANDED_MATERIAL_FILTERED_WMO_COVERAGE_PROFILE}\ngameobject-scope\t{SELECTED_GEOMETRY_GAMEOBJECT_SCOPE}\nmodel-scope\t{SELECTED_GEOMETRY_MODEL_SCOPE}\n"
            )
        } else if self.is_v41() {
            format!(
                "collision-profile\t{SOURCE_EXPANDED_MATERIAL_FILTERED_WMO_COLLISION_PROFILE}\ncollision-domain\tall-available-maps\ncollision-coverage\t{SOURCE_EXPANDED_MATERIAL_FILTERED_WMO_COVERAGE_PROFILE}\ngameobject-scope\t{SELECTED_GEOMETRY_GAMEOBJECT_SCOPE}\nmodel-scope\t{SELECTED_GEOMETRY_MODEL_SCOPE}\n"
            )
        } else if self.is_v40() {
            format!(
                "collision-profile\t{MODEL_HEAVY_MATERIAL_FILTERED_WMO_COLLISION_PROFILE}\ncollision-domain\tall-available-maps\ncollision-coverage\t{MODEL_HEAVY_MATERIAL_FILTERED_WMO_COVERAGE_PROFILE}\ngameobject-scope\t{SELECTED_GEOMETRY_GAMEOBJECT_SCOPE}\nmodel-scope\t{SELECTED_GEOMETRY_MODEL_SCOPE}\n"
            )
        } else if self.is_v39() {
            format!(
                "collision-profile\t{READ_HEAVY_MATERIAL_FILTERED_WMO_COLLISION_PROFILE}\ncollision-domain\tall-available-maps\ncollision-coverage\t{READ_HEAVY_MATERIAL_FILTERED_WMO_COVERAGE_PROFILE}\ngameobject-scope\t{SELECTED_GEOMETRY_GAMEOBJECT_SCOPE}\nmodel-scope\t{SELECTED_GEOMETRY_MODEL_SCOPE}\n"
            )
        } else if self.is_v38() {
            format!(
                "collision-profile\t{SOURCE_HEAVY_MATERIAL_FILTERED_WMO_COLLISION_PROFILE}\ncollision-domain\tall-available-maps\ncollision-coverage\t{SOURCE_HEAVY_MATERIAL_FILTERED_WMO_COVERAGE_PROFILE}\ngameobject-scope\t{SELECTED_GEOMETRY_GAMEOBJECT_SCOPE}\nmodel-scope\t{SELECTED_GEOMETRY_MODEL_SCOPE}\n"
            )
        } else if self.is_v37() {
            format!(
                "collision-profile\t{DEPENDENCY_HEAVY_MATERIAL_FILTERED_WMO_COLLISION_PROFILE}\ncollision-domain\tall-available-maps\ncollision-coverage\t{DEPENDENCY_HEAVY_MATERIAL_FILTERED_WMO_COVERAGE_PROFILE}\ngameobject-scope\t{SELECTED_GEOMETRY_GAMEOBJECT_SCOPE}\nmodel-scope\t{SELECTED_GEOMETRY_MODEL_SCOPE}\n"
            )
        } else if self.is_v36() {
            format!(
                "collision-profile\t{MATERIAL_FILTERED_WMO_COLLISION_PROFILE}\ncollision-domain\tall-available-maps\ncollision-coverage\t{MATERIAL_FILTERED_WMO_COVERAGE_PROFILE}\ngameobject-scope\t{SELECTED_GEOMETRY_GAMEOBJECT_SCOPE}\nmodel-scope\t{SELECTED_GEOMETRY_MODEL_SCOPE}\n"
            )
        } else if self.is_v35() {
            format!(
                "collision-profile\t{UNSELECTED_WMO_COLLISION_PROFILE}\ncollision-domain\tall-available-maps\ncollision-coverage\t{UNSELECTED_WMO_COVERAGE_PROFILE}\ngameobject-scope\t{SELECTED_GEOMETRY_GAMEOBJECT_SCOPE}\nmodel-scope\t{SELECTED_GEOMETRY_MODEL_SCOPE}\n"
            )
        } else if self.is_v34() {
            format!(
                "collision-profile\t{EMPTY_WMO_COLLISION_PROFILE}\ncollision-domain\tall-available-maps\ncollision-coverage\t{EMPTY_WMO_COVERAGE_PROFILE}\ngameobject-scope\t{SELECTED_GEOMETRY_GAMEOBJECT_SCOPE}\nmodel-scope\t{SELECTED_GEOMETRY_MODEL_SCOPE}\n"
            )
        } else if self.is_v33() {
            format!(
                "collision-profile\t{VACATED_WMO_COLLISION_PROFILE}\ncollision-domain\tall-available-maps\ncollision-coverage\t{VACATED_WMO_COVERAGE_PROFILE}\ngameobject-scope\t{SELECTED_GEOMETRY_GAMEOBJECT_SCOPE}\nmodel-scope\t{SELECTED_GEOMETRY_MODEL_SCOPE}\n"
            )
        } else if self.is_v32() {
            format!(
                "collision-profile\t{MODEL_ROOT_ABSENT_REQUEST_CAPACITY_GEOMETRY_COLLISION_PROFILE}\ncollision-domain\tall-available-maps\ncollision-coverage\t{MODEL_ROOT_ABSENT_REQUEST_CAPACITY_GEOMETRY_COVERAGE_PROFILE}\ngameobject-scope\t{SELECTED_GEOMETRY_GAMEOBJECT_SCOPE}\nmodel-scope\t{SELECTED_GEOMETRY_MODEL_SCOPE}\n"
            )
        } else if self.is_v31() {
            format!(
                "collision-profile\t{MODEL_ROOT_ABSENT_GEOMETRY_COLLISION_PROFILE}\ncollision-domain\tall-available-maps\ncollision-coverage\t{MODEL_ROOT_ABSENT_GEOMETRY_COVERAGE_PROFILE}\ngameobject-scope\t{SELECTED_GEOMETRY_GAMEOBJECT_SCOPE}\nmodel-scope\t{SELECTED_GEOMETRY_MODEL_SCOPE}\n"
            )
        } else if self.is_v30() {
            format!(
                "collision-profile\t{ROOT_ABSENT_GEOMETRY_COLLISION_PROFILE}\ncollision-domain\tall-available-maps\ncollision-coverage\t{ROOT_ABSENT_GEOMETRY_COVERAGE_PROFILE}\ngameobject-scope\t{SELECTED_GEOMETRY_GAMEOBJECT_SCOPE}\nmodel-scope\t{SELECTED_GEOMETRY_MODEL_SCOPE}\n"
            )
        } else if self.is_v29() {
            format!(
                "collision-profile\t{AVAILABLE_GEOMETRY_COLLISION_PROFILE}\ncollision-domain\tall-available-maps\ncollision-coverage\t{AVAILABLE_GEOMETRY_COVERAGE_PROFILE}\ngameobject-scope\t{SELECTED_GEOMETRY_GAMEOBJECT_SCOPE}\nmodel-scope\t{SELECTED_GEOMETRY_MODEL_SCOPE}\n"
            )
        } else if self.has_selected_geometry() {
            let id = self.map_domain.ok_or("product_plan_map_domain")?;
            format!(
                "collision-profile\t{SELECTED_GEOMETRY_COLLISION_PROFILE}\ncollision-domain\tmap:{id}\ncollision-coverage\t{SELECTED_GEOMETRY_COVERAGE_PROFILE}\ngameobject-scope\t{SELECTED_GEOMETRY_GAMEOBJECT_SCOPE}\nmodel-scope\t{SELECTED_GEOMETRY_MODEL_SCOPE}\n"
            )
        } else if self.is_v14() || self.is_v15() || self.is_v18() {
            let selected = self
                .map_domain
                .map_or_else(|| "all-available-maps".to_owned(), |id| format!("map:{id}"));
            let profile = if self.is_v15() || self.is_v18() {
                COMPATIBLE_COLLISION_VERIFY_PROFILE_70450
            } else {
                COMPATIBLE_COLLISION_VERIFY_PROFILE
            };
            format!(
                "collision-profile\t{profile}\ncollision-domain\t{selected}\ncollision-coverage\tcompatible-collision-coverage-v1\n"
            )
        } else {
            map_domain::render(self.map_domain)
        };
        let mut output = format!(
            "forever-product\t{}\nbuild\t{}\nlocale\t{}\nindex\t{index}\nroot\t{root}\n{schema_catalog}{decryption}{partial_encrypted_sections}{collision_storage}{map_domain}components\t{components}\nterrain\t{}\npack\t{}\nnavigation\t{}\nbig-base-unit\t{}\nskip-liquids\t{}\nmaximum-slope\t{}\nnon-steep-slope\t{}\n",
            self.version(),
            self.build,
            self.locale.name(),
            if self.is_any_available_terrain() && self.terrain == Selection::All {
                "all-available-maps".into()
            } else {
                self.terrain.render()
            },
            self.pack,
            self.navigation.render(),
            self.big_base_unit,
            self.skip_liquids,
            self.maximum_slope,
            self.non_steep_slope,
        );
        if let Some(connections) = &self.offmesh {
            writeln!(output, "offmesh\t{OFFMESH_PROFILE}")
                .expect("writing into String cannot fail");
            for connection in connections {
                connection.render(&mut output);
            }
        }
        for entry in &self.db2 {
            writeln!(output, "db2\t{}\t{}", entry.name, entry.id)
                .expect("writing into String cannot fail");
        }
        Ok(output)
    }
}

fn field<'a>(
    lines: &mut impl Iterator<Item = &'a str>,
    key: &str,
) -> Result<&'a str, &'static str> {
    let line = lines.next().ok_or("product_plan_format")?;
    let (name, value) = line.split_once('\t').ok_or("product_plan_format")?;
    if name != key || value.is_empty() || value.contains('\t') {
        return Err("product_plan_format");
    }
    Ok(value)
}
fn decimal(value: &str) -> Result<u32, &'static str> {
    if value.is_empty()
        || !value.bytes().all(|byte| byte.is_ascii_digit())
        || (value.len() > 1 && value.starts_with('0'))
    {
        return Err("product_plan_decimal");
    }
    value.parse().map_err(|_| "product_plan_decimal")
}
fn boolean(value: &str) -> Result<bool, &'static str> {
    match value {
        "true" => Ok(true),
        "false" => Ok(false),
        _ => Err("product_plan_boolean"),
    }
}

#[cfg(test)]
mod keys_tests;
#[cfg(test)]
mod tests;

#[cfg(test)]
mod disk_tests;
#[cfg(test)]
mod map_domain_tests;
#[cfg(test)]
pub(crate) mod offmesh_tests;
#[cfg(test)]
mod v10_tests;
#[cfg(test)]
mod v9_tests;

#[cfg(test)]
mod v11_tests;

#[cfg(test)]
mod v12_tests;

#[cfg(test)]
mod v13_tests;

#[cfg(test)]
pub(crate) mod v14_tests;

#[cfg(test)]
pub(crate) mod v15_tests;

#[cfg(test)]
mod v16_tests;

#[cfg(test)]
mod v17_tests;

#[cfg(test)]
mod v18_tests;

#[cfg(test)]
mod v19_tests;

#[cfg(test)]
mod v20_tests;

#[cfg(test)]
mod v21_tests;

#[cfg(test)]
mod v22_tests;
#[cfg(test)]
mod v23_tests;

#[cfg(test)]
mod v24_tests;

#[cfg(test)]
mod v25_tests;

#[cfg(test)]
mod v26_tests;

#[cfg(test)]
mod v27_tests;

#[cfg(test)]
mod v28_tests;

#[cfg(test)]
mod v29_tests;

#[cfg(test)]
mod v30_tests;

#[cfg(test)]
mod v31_tests;

#[cfg(test)]
mod v32_tests;

#[cfg(test)]
mod v33_tests;

#[cfg(test)]
mod v34_tests;

#[cfg(test)]
mod v35_tests;

#[cfg(test)]
mod v36_tests;

#[cfg(test)]
mod v37_tests;

#[cfg(test)]
mod v38_tests;

#[cfg(test)]
mod v39_tests;

#[cfg(test)]
mod v40_tests;

#[cfg(test)]
mod v41_tests;

#[cfg(test)]
mod v42_tests;

#[cfg(test)]
mod v43_tests;

#[cfg(test)]
mod v44_tests;

#[cfg(test)]
mod v45_tests;

#[cfg(test)]
mod v46_tests;

#[cfg(test)]
mod v47_tests;

#[cfg(test)]
mod v48_tests;

#[cfg(test)]
mod v49_tests;

#[cfg(test)]
mod v50_tests;
#[cfg(test)]
mod v51_tests;
