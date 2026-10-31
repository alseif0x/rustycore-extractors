// SPDX-License-Identifier: GPL-3.0-or-later
// Authored 2026-10-19: build-70450 selected-map collision geometry product.
// Modified 2026-10-24: explicit all-active-map selection and coverage versions 2/3/4/5.
// Modified 2026-10-25: report vacated WMOs and intrinsically empty WMO roots in v6/v7.
// Modified 2026-10-26: report unselected WMO roots in v8, material-filtered WMOs in v9,
// dependency-/source-/read-heavy profiles in v10/v11/v12, and model-heavy resources in v13.
// Modified 2026-10-27: SourceExpandedV1 adds v14; ModelOutputExpandedV1 adds v15
// with the expanded staged model-count and model-byte limits; ModelBytesExpandedV1 adds v16
// with the expanded staged model-byte and placement capacities; SourceExpandedV2 adds v17
// with the expanded source-byte limit and unchanged v16 staged limits; PlacementExpandedV1
// adds v18 and raises only aggregate staged placements to 256 MiB while keeping 64 MiB per map;
// AssemblyExpandedV1 adds v19 with the same resource/placement limits and a 256 MiB assembled domain.
// GameObjectExpandedV1 adds v20 GODI facts, Dtree publication and a union model inventory.
//! Linux v22/v28 publish selected-domain VMap geometry; v29 publishes every
//! admitted active map through the same runner. v30 additionally declares only
//! runner-proven ROOT omissions and assembles just present maps. v31 can also
//! declare ROOT-absent M2 doodads nested in WMO while retaining the parent WMO,
//! its placement and every present model. Direct M2 and all other sources stay
//! strict. v32 separately enables the resident RequestHeavyV1 request ceiling;
//! v33 reports producer-reported vacated WMOs without claiming independent proof
//! of ROOT absence. v34 also reports intrinsically empty WMO roots separately;
//! v35 additionally reports producer-reported unselected WMOs, and v36-v38 add
//! producer-reported material-filtered WMOs under request-, dependency- and
//! source-heavy resource profiles respectively. V39 uses the read-heavy profile,
//! v40 the model-heavy profile, v41 the source-expanded profile, v42 the
//! model-output-expanded profile and v43 the model-bytes-expanded profile. V42
//! and v43 raise the local staged model-count limit to 65,536 and use distinct
//! model-count and model-byte diagnostics; v44 adds source-expanded v2, v45 expands
//! aggregate placements, and v46 independently expands the assembled domain.
//! Omission-list limits remain unchanged.
//! Callbacks remain provisional until the all-or-error run completes.

use crate::{Error, driver::disk::Preflight, plan::DiskBudget};
use forever_casc::{
    collision::{
        Class, Status, geometry,
        plan::MapSelection,
        resources, run,
        world::{self, TableFacts},
    },
    integrity,
    package::{Component, Context, Limits, Run},
};
use forever_vmap::{RawM2, assembly, placement, vmo, wmo::raw};
use std::{collections::BTreeMap, str};

pub(crate) const COVERAGE_PATH: &str = "vmaps/selected-geometry-coverage-v1.bin";
pub(crate) const AVAILABLE_COVERAGE_PATH: &str = "vmaps/available-geometry-coverage-v2.bin";
pub(crate) const ROOT_ABSENT_COVERAGE_PATH: &str =
    "vmaps/available-root-absent-geometry-coverage-v3.bin";
pub(crate) const MODEL_ROOT_ABSENT_COVERAGE_PATH: &str =
    "vmaps/available-model-root-omissions-geometry-coverage-v4.bin";
pub(crate) const REQUEST_CAPACITY_COVERAGE_PATH: &str =
    "vmaps/available-request-capacity-geometry-coverage-v5.bin";
pub(crate) const VACATED_WMO_COVERAGE_PATH: &str =
    "vmaps/available-vacated-wmo-geometry-coverage-v6.bin";
pub(crate) const EMPTY_WMO_COVERAGE_PATH: &str =
    "vmaps/available-empty-wmo-geometry-coverage-v7.bin";
pub(crate) const UNSELECTED_WMO_COVERAGE_PATH: &str =
    "vmaps/available-unselected-wmo-geometry-coverage-v8.bin";
pub(crate) const MATERIAL_FILTERED_WMO_COVERAGE_PATH: &str =
    "vmaps/available-material-filtered-wmo-geometry-coverage-v9.bin";
pub(crate) const DEPENDENCY_HEAVY_MATERIAL_FILTERED_WMO_COVERAGE_PATH: &str =
    "vmaps/available-dependency-heavy-material-filtered-wmo-geometry-coverage-v10.bin";
pub(crate) const SOURCE_HEAVY_MATERIAL_FILTERED_WMO_COVERAGE_PATH: &str =
    "vmaps/available-source-heavy-material-filtered-wmo-geometry-coverage-v11.bin";
pub(crate) const READ_HEAVY_MATERIAL_FILTERED_WMO_COVERAGE_PATH: &str =
    "vmaps/available-read-heavy-material-filtered-wmo-geometry-coverage-v12.bin";
pub(crate) const MODEL_HEAVY_MATERIAL_FILTERED_WMO_COVERAGE_PATH: &str =
    "vmaps/available-model-heavy-material-filtered-wmo-geometry-coverage-v13.bin";
pub(crate) const SOURCE_EXPANDED_MATERIAL_FILTERED_WMO_COVERAGE_PATH: &str =
    "vmaps/available-source-expanded-material-filtered-wmo-geometry-coverage-v14.bin";
pub(crate) const MODEL_OUTPUT_EXPANDED_MATERIAL_FILTERED_WMO_COVERAGE_PATH: &str =
    "vmaps/available-model-output-expanded-material-filtered-wmo-geometry-coverage-v15.bin";
pub(crate) const MODEL_BYTES_EXPANDED_MATERIAL_FILTERED_WMO_COVERAGE_PATH: &str =
    "vmaps/available-model-bytes-expanded-material-filtered-wmo-geometry-coverage-v16.bin";
pub(crate) const SOURCE_EXPANDED_V2_MATERIAL_FILTERED_WMO_COVERAGE_PATH: &str =
    "vmaps/available-source-expanded-v2-material-filtered-wmo-geometry-coverage-v17.bin";
pub(crate) const PLACEMENT_EXPANDED_MATERIAL_FILTERED_WMO_COVERAGE_PATH: &str =
    "vmaps/available-placement-expanded-material-filtered-wmo-geometry-coverage-v18.bin";
pub(crate) const ASSEMBLY_EXPANDED_MATERIAL_FILTERED_WMO_COVERAGE_PATH: &str =
    "vmaps/available-assembly-expanded-material-filtered-wmo-geometry-coverage-v19.bin";
pub(crate) const GAMEOBJECT_EXPANDED_MATERIAL_FILTERED_WMO_COVERAGE_PATH: &str =
    "vmaps/available-gameobject-expanded-material-filtered-wmo-geometry-coverage-v20.bin";
pub(crate) const GAMEOBJECT_MISSING_ROOT_MATERIAL_FILTERED_WMO_COVERAGE_PATH: &str =
    "vmaps/available-gameobject-missing-root-material-filtered-wmo-geometry-coverage-v21.bin";
pub(crate) const GAMEOBJECT_ROOT_ENCRYPTED_UNAVAILABLE_MATERIAL_FILTERED_WMO_COVERAGE_PATH: &str = "vmaps/available-gameobject-root-encrypted-unavailable-material-filtered-wmo-geometry-coverage-v22.bin";
pub(crate) const DB2_TERRAIN_LIQUID_VMAP_COVERAGE_PATH: &str = "vmaps/available-db2-terrain-liquid-gameobject-root-encrypted-unavailable-geometry-coverage-v1.bin";
pub(crate) const GAMEOBJECT_MISSING_ROOT_MAX: usize =
    forever_casc::collision::gameobjects::MAX_CANDIDATE_REQUESTS;
pub(crate) const GAMEOBJECT_ENCRYPTED_UNAVAILABLE_MAX: usize =
    forever_casc::collision::gameobjects::MAX_CANDIDATE_REQUESTS;
pub(crate) const GAMEOBJECT_MODELS_DTREE_PATH: &str = "vmaps/GameObjectModels.dtree";
pub(crate) const COVERAGE_MAGIC: &[u8; 8] = b"RCVGEO1\0";
pub(crate) const PROFILE: &str = "linux70450-selected-map-geometry-v1";
pub(crate) const AVAILABLE_COVERAGE_PROFILE: &str = "linux70450-available-map-geometry-v1";
pub(crate) const ROOT_ABSENT_COVERAGE_PROFILE: &str = "linux70450-available-map-root-omissions-v1";
pub(crate) const MODEL_ROOT_ABSENT_COVERAGE_PROFILE: &str =
    "linux70450-available-map-model-root-omissions-v1";
pub(crate) const REQUEST_CAPACITY_COVERAGE_PROFILE: &str =
    "linux70450-available-map-model-root-omissions-request-capacity-v1";
const VACATED_WMO_CODEC_PROFILE: &str = "linux70450-available-vacated-wmo-v1";
pub(crate) const VACATED_WMO_COMPLETION_PROFILE: &str =
    "linux70450-available-vacated-wmo-collision-v1";
const VACATED_WMO_DOMAIN_PROFILE: &str = "available-vacated-wmo-v1";
const EMPTY_WMO_CODEC_PROFILE: &str = "linux70450-available-empty-wmo-v1";
pub(crate) const EMPTY_WMO_COMPLETION_PROFILE: &str = "linux70450-available-empty-wmo-collision-v1";
const EMPTY_WMO_DOMAIN_PROFILE: &str = "available-empty-wmo-v1";
const UNSELECTED_WMO_CODEC_PROFILE: &str = "linux70450-available-unselected-wmo-v1";
pub(crate) const UNSELECTED_WMO_COMPLETION_PROFILE: &str =
    "linux70450-available-unselected-wmo-collision-v1";
const UNSELECTED_WMO_DOMAIN_PROFILE: &str = "available-unselected-wmo-v1";
const MATERIAL_FILTERED_WMO_CODEC_PROFILE: &str = "linux70450-material-filtered-wmo-v1";
pub(crate) const MATERIAL_FILTERED_WMO_COMPLETION_PROFILE: &str =
    "linux70450-available-material-filtered-wmo-collision-v1";
const MATERIAL_FILTERED_WMO_DOMAIN_PROFILE: &str = "material-filtered-wmo-v1";
const DEPENDENCY_HEAVY_MATERIAL_FILTERED_WMO_CODEC_PROFILE: &str =
    "linux70450-dependency-heavy-material-filtered-wmo-v1";
pub(crate) const DEPENDENCY_HEAVY_MATERIAL_FILTERED_WMO_COMPLETION_PROFILE: &str =
    "linux70450-available-dependency-heavy-material-filtered-wmo-collision-v1";
const DEPENDENCY_HEAVY_MATERIAL_FILTERED_WMO_DOMAIN_PROFILE: &str =
    "dependency-heavy-material-filtered-wmo-v1";
pub(crate) const SOURCE_HEAVY_MATERIAL_FILTERED_WMO_CODEC_PROFILE: &str =
    "linux70450-source-heavy-material-filtered-wmo-v1";
pub(crate) const SOURCE_HEAVY_MATERIAL_FILTERED_WMO_COMPLETION_PROFILE: &str =
    "linux70450-available-source-heavy-material-filtered-wmo-collision-v1";
const SOURCE_HEAVY_MATERIAL_FILTERED_WMO_DOMAIN_PROFILE: &str =
    "source-heavy-material-filtered-wmo-v1";
pub(crate) const READ_HEAVY_MATERIAL_FILTERED_WMO_CODEC_PROFILE: &str =
    "linux70450-read-heavy-material-filtered-wmo-v1";
pub(crate) const READ_HEAVY_MATERIAL_FILTERED_WMO_COMPLETION_PROFILE: &str =
    "linux70450-available-read-heavy-material-filtered-wmo-collision-v1";
const READ_HEAVY_MATERIAL_FILTERED_WMO_DOMAIN_PROFILE: &str = "read-heavy-material-filtered-wmo-v1";
const MODEL_HEAVY_MATERIAL_FILTERED_WMO_CODEC_PROFILE: &str =
    "linux70450-model-heavy-material-filtered-wmo-v1";
pub(crate) const MODEL_HEAVY_MATERIAL_FILTERED_WMO_COMPLETION_PROFILE: &str =
    "linux70450-available-model-heavy-material-filtered-wmo-collision-v1";
const MODEL_HEAVY_MATERIAL_FILTERED_WMO_DOMAIN_PROFILE: &str =
    "model-heavy-material-filtered-wmo-v1";
pub(crate) const SOURCE_EXPANDED_MATERIAL_FILTERED_WMO_CODEC_PROFILE: &str =
    "linux70450-source-expanded-material-filtered-wmo-v1";
pub(crate) const SOURCE_EXPANDED_MATERIAL_FILTERED_WMO_COMPLETION_PROFILE: &str =
    "linux70450-available-source-expanded-material-filtered-wmo-collision-v1";
const SOURCE_EXPANDED_MATERIAL_FILTERED_WMO_DOMAIN_PROFILE: &str =
    "source-expanded-material-filtered-wmo-v1";
pub(crate) const MODEL_OUTPUT_EXPANDED_MATERIAL_FILTERED_WMO_CODEC_PROFILE: &str =
    "linux70450-model-output-expanded-material-filtered-wmo-v1";
pub(crate) const MODEL_OUTPUT_EXPANDED_MATERIAL_FILTERED_WMO_COMPLETION_PROFILE: &str =
    "linux70450-available-model-output-expanded-material-filtered-wmo-collision-v1";
const MODEL_OUTPUT_EXPANDED_MATERIAL_FILTERED_WMO_DOMAIN_PROFILE: &str =
    "model-output-expanded-material-filtered-wmo-v1";
pub(crate) const MODEL_BYTES_EXPANDED_MATERIAL_FILTERED_WMO_CODEC_PROFILE: &str =
    "linux70450-model-bytes-expanded-material-filtered-wmo-v1";
pub(crate) const MODEL_BYTES_EXPANDED_MATERIAL_FILTERED_WMO_COMPLETION_PROFILE: &str =
    "linux70450-available-model-bytes-expanded-material-filtered-wmo-collision-v1";
const MODEL_BYTES_EXPANDED_MATERIAL_FILTERED_WMO_DOMAIN_PROFILE: &str =
    "model-bytes-expanded-material-filtered-wmo-v1";
pub(crate) const SOURCE_EXPANDED_V2_MATERIAL_FILTERED_WMO_CODEC_PROFILE: &str =
    "linux70450-source-expanded-v2-material-filtered-wmo-v1";
pub(crate) const SOURCE_EXPANDED_V2_MATERIAL_FILTERED_WMO_COMPLETION_PROFILE: &str =
    "linux70450-available-source-expanded-v2-material-filtered-wmo-collision-v1";
const SOURCE_EXPANDED_V2_MATERIAL_FILTERED_WMO_DOMAIN_PROFILE: &str =
    "source-expanded-v2-material-filtered-wmo-v1";
pub(crate) const PLACEMENT_EXPANDED_MATERIAL_FILTERED_WMO_CODEC_PROFILE: &str =
    "linux70450-placement-expanded-material-filtered-wmo-v1";
pub(crate) const PLACEMENT_EXPANDED_MATERIAL_FILTERED_WMO_COMPLETION_PROFILE: &str =
    "linux70450-available-placement-expanded-material-filtered-wmo-collision-v1";
const PLACEMENT_EXPANDED_MATERIAL_FILTERED_WMO_DOMAIN_PROFILE: &str =
    "placement-expanded-material-filtered-wmo-v1";
pub(crate) const ASSEMBLY_EXPANDED_MATERIAL_FILTERED_WMO_CODEC_PROFILE: &str =
    "linux70450-assembly-expanded-material-filtered-wmo-v1";
pub(crate) const ASSEMBLY_EXPANDED_MATERIAL_FILTERED_WMO_COMPLETION_PROFILE: &str =
    "linux70450-available-assembly-expanded-material-filtered-wmo-collision-v1";
const ASSEMBLY_EXPANDED_MATERIAL_FILTERED_WMO_DOMAIN_PROFILE: &str =
    "assembly-expanded-material-filtered-wmo-v1";
const GAMEOBJECT_EXPANDED_MATERIAL_FILTERED_WMO_CODEC_PROFILE: &str =
    "linux70450-gameobject-expanded-material-filtered-wmo-v1";
pub(crate) const GAMEOBJECT_EXPANDED_MATERIAL_FILTERED_WMO_COMPLETION_PROFILE: &str =
    "linux70450-available-gameobject-expanded-material-filtered-wmo-collision-v1";
const GAMEOBJECT_EXPANDED_MATERIAL_FILTERED_WMO_DOMAIN_PROFILE: &str =
    "gameobject-expanded-material-filtered-wmo-v1";
const GAMEOBJECT_MISSING_ROOT_CODEC_PROFILE: &str =
    "linux70450-gameobject-missing-root-material-filtered-wmo-v1";
pub(crate) const GAMEOBJECT_MISSING_ROOT_COMPLETION_PROFILE: &str =
    "linux70450-available-gameobject-missing-root-material-filtered-wmo-collision-v1";
const GAMEOBJECT_MISSING_ROOT_DOMAIN_PROFILE: &str =
    "gameobject-missing-root-material-filtered-wmo-v1";
const GAMEOBJECT_MISSING_ROOT_COVERAGE_PROFILE: &str =
    "linux70450-gameobject-missing-root-material-filtered-wmo-coverage-v21";
const GAMEOBJECT_ROOT_ENCRYPTED_UNAVAILABLE_CODEC_PROFILE: &str =
    "linux70450-gameobject-root-encrypted-unavailable-material-filtered-wmo-v1";
pub(crate) const GAMEOBJECT_ROOT_ENCRYPTED_UNAVAILABLE_COMPLETION_PROFILE: &str =
    "linux70450-available-gameobject-root-encrypted-unavailable-material-filtered-wmo-collision-v1";
const GAMEOBJECT_ROOT_ENCRYPTED_UNAVAILABLE_DOMAIN_PROFILE: &str =
    "gameobject-root-encrypted-unavailable-material-filtered-wmo-v1";
const GAMEOBJECT_ROOT_ENCRYPTED_UNAVAILABLE_COVERAGE_PROFILE: &str =
    "linux70450-gameobject-root-encrypted-unavailable-material-filtered-wmo-coverage-v22";
const DB2_TERRAIN_LIQUID_VMAP_CODEC_PROFILE: &str =
    "linux70450-db2-terrain-liquid-vmap-geometry-v1";
const GAMEOBJECT_ROOT_ENCRYPTED_UNAVAILABLE_SCOPE_PROFILE: &str =
    "available-fdid-direct-root-absent-or-encrypted-unavailable-v1";
const GAMEOBJECT_ROOT_ENCRYPTED_UNAVAILABLE_MODEL_SCOPE_PROFILE: &str =
    "placement-and-gameobject-with-root-absent-or-encrypted-unavailable-v1";
const GAMEOBJECT_SCOPE_PROFILE: &str = "available-fdid-direct-v1";
const GAMEOBJECT_MODEL_SCOPE_PROFILE: &str = "placement-and-gameobject-v1";
const GAMEOBJECT_MISSING_ROOT_SCOPE_PROFILE: &str = "available-fdid-direct-missing-root-v1";
const GAMEOBJECT_MISSING_ROOT_MODEL_SCOPE_PROFILE: &str =
    "placement-and-gameobject-with-missing-root-v1";
const COVERAGE_VERSION: u32 = 1;
const AVAILABLE_COVERAGE_VERSION: u32 = 2;
const ROOT_ABSENT_COVERAGE_VERSION: u32 = 3;
const MODEL_ROOT_ABSENT_COVERAGE_VERSION: u32 = 4;
const REQUEST_CAPACITY_COVERAGE_VERSION: u32 = 5;
const VACATED_WMO_COVERAGE_VERSION: u32 = 6;
const EMPTY_WMO_COVERAGE_VERSION: u32 = 7;
const UNSELECTED_WMO_COVERAGE_VERSION: u32 = 8;
const MATERIAL_FILTERED_WMO_COVERAGE_VERSION: u32 = 9;
const DEPENDENCY_HEAVY_MATERIAL_FILTERED_WMO_COVERAGE_VERSION: u32 = 10;
const SOURCE_HEAVY_MATERIAL_FILTERED_WMO_COVERAGE_VERSION: u32 = 11;
const READ_HEAVY_MATERIAL_FILTERED_WMO_COVERAGE_VERSION: u32 = 12;
const MODEL_HEAVY_MATERIAL_FILTERED_WMO_COVERAGE_VERSION: u32 = 13;
const SOURCE_EXPANDED_MATERIAL_FILTERED_WMO_COVERAGE_VERSION: u32 = 14;
const MODEL_OUTPUT_EXPANDED_MATERIAL_FILTERED_WMO_COVERAGE_VERSION: u32 = 15;
const MODEL_BYTES_EXPANDED_MATERIAL_FILTERED_WMO_COVERAGE_VERSION: u32 = 16;
const SOURCE_EXPANDED_V2_MATERIAL_FILTERED_WMO_COVERAGE_VERSION: u32 = 17;
const PLACEMENT_EXPANDED_MATERIAL_FILTERED_WMO_COVERAGE_VERSION: u32 = 18;
const ASSEMBLY_EXPANDED_MATERIAL_FILTERED_WMO_COVERAGE_VERSION: u32 = 19;
const GAMEOBJECT_EXPANDED_MATERIAL_FILTERED_WMO_COVERAGE_VERSION: u32 = 20;
const GAMEOBJECT_MISSING_ROOT_COVERAGE_VERSION: u32 = 21;
const GAMEOBJECT_ROOT_ENCRYPTED_UNAVAILABLE_COVERAGE_VERSION: u32 = 22;
const ROOT_ABSENT_DOMAIN_PROFILE: &str = "available-map-root-omissions-v1";
const MODEL_ROOT_ABSENT_DOMAIN_PROFILE: &str = "available-map-model-root-omissions-v1";
const REQUEST_CAPACITY_DOMAIN_PROFILE: &str =
    "available-map-model-root-omissions-request-capacity-v1";
const COVERAGE_FACTS: u32 = 0x3ff;
const COVERAGE_MAX_BYTES: usize = 128 * 1024 * 1024;
const COVERAGE_MAX_MISSING_M2: usize = 4096;
const COVERAGE_MAX_MISSING_WMO: usize = 4096;
const COVERAGE_MAX_EMPTY_WMO: usize = 4096;
const COVERAGE_MAX_UNSELECTED_WMO: usize = 4096;
const COVERAGE_MAX_MATERIAL_FILTERED_WMO: usize = 4096;
pub(crate) const COVERAGE_MAX_MAPS: usize = 4096;
pub(crate) const COVERAGE_MAX_ARTIFACTS: usize = 65_535;
pub(crate) const PACKAGE_MAX_FILES: usize = COVERAGE_MAX_ARTIFACTS + 1;
const MAX_STAGED_MODEL_BYTES: usize = 384 * 1024 * 1024;
const MAX_STAGED_PLACEMENT_BYTES: usize = 128 * 1024 * 1024;
const MAX_STAGED_PLACEMENT_BYTES_V45: usize = 256 * 1024 * 1024;

fn staged_placement_limit(
    resource_profile: resources::Profile,
    plan_limit: u64,
) -> Result<usize, Error> {
    let profile_limit = if matches!(
        resource_profile,
        resources::Profile::PlacementExpandedV1
            | resources::Profile::AssemblyExpandedV1
            | resources::Profile::GameObjectExpandedV1
            | resources::Profile::GameObjectMissingRootExpandedV1
            | resources::Profile::GameObjectRootAndEncryptedUnavailableExpandedV1
    ) {
        MAX_STAGED_PLACEMENT_BYTES_V45
    } else {
        MAX_STAGED_PLACEMENT_BYTES
    };
    Ok(as_usize(plan_limit)?.min(profile_limit))
}
pub(crate) const MAX_PLACEMENT_RECORDS: usize = 1_000_000;
pub(crate) const MAX_ASSEMBLY_MEMORY_BYTES: usize = 512 * 1024 * 1024;
pub(crate) const MAX_ASSEMBLY_OUTPUT_BYTES: usize = 128 * 1024 * 1024;
pub(crate) const MAX_ASSEMBLED_DOMAIN_BYTES: u64 = 128 * 1024 * 1024;
pub(crate) const MAX_ASSEMBLED_DOMAIN_BYTES_V46: u64 = 256 * 1024 * 1024;
const ASSEMBLED_DOMAIN_LIMIT_V46: &str = "product_selected_geometry_assembled_domain_limit";
const LIMIT: &str = "product_selected_geometry_limit";
const PLACEMENT_BYTES_LIMIT: &str = "product_selected_geometry_placement_bytes_limit";
const PLACEMENT_MAP_LIMIT: &str = "product_selected_geometry_placement_map_limit";
const PLACEMENT_DOMAIN_LIMIT: &str = "product_selected_geometry_domain_limit";
const MODEL_COUNT_LIMIT: &str = "product_selected_geometry_model_count_limit";
const MODEL_BYTES_LIMIT: &str = "product_selected_geometry_model_bytes_limit";
const SCHEMA: &str = "product_selected_geometry_coverage_schema";

#[derive(Clone, Copy)]
struct PlacementErrorCodes {
    aggregate: &'static str,
    map: &'static str,
    domain: &'static str,
    domain_count: &'static str,
}

fn placement_error_codes(resource_profile: resources::Profile) -> PlacementErrorCodes {
    if matches!(
        resource_profile,
        resources::Profile::PlacementExpandedV1
            | resources::Profile::AssemblyExpandedV1
            | resources::Profile::GameObjectExpandedV1
            | resources::Profile::GameObjectMissingRootExpandedV1
            | resources::Profile::GameObjectRootAndEncryptedUnavailableExpandedV1
    ) {
        PlacementErrorCodes {
            aggregate: PLACEMENT_BYTES_LIMIT,
            map: PLACEMENT_MAP_LIMIT,
            domain: PLACEMENT_DOMAIN_LIMIT,
            domain_count: PLACEMENT_DOMAIN_LIMIT,
        }
    } else {
        PlacementErrorCodes {
            aggregate: LIMIT,
            map: LIMIT,
            domain: "product_selected_geometry_domain",
            domain_count: LIMIT,
        }
    }
}

fn assembled_domain_limit(resource_profile: resources::Profile, plan_limit: u64) -> u64 {
    let profile_limit = if matches!(
        resource_profile,
        resources::Profile::AssemblyExpandedV1
            | resources::Profile::GameObjectExpandedV1
            | resources::Profile::GameObjectMissingRootExpandedV1
            | resources::Profile::GameObjectRootAndEncryptedUnavailableExpandedV1
    ) {
        MAX_ASSEMBLED_DOMAIN_BYTES_V46
    } else {
        MAX_ASSEMBLED_DOMAIN_BYTES
    };
    plan_limit.min(profile_limit)
}

fn assembled_domain_error(resource_profile: resources::Profile) -> &'static str {
    if matches!(
        resource_profile,
        resources::Profile::AssemblyExpandedV1
            | resources::Profile::GameObjectExpandedV1
            | resources::Profile::GameObjectMissingRootExpandedV1
            | resources::Profile::GameObjectRootAndEncryptedUnavailableExpandedV1
    ) {
        ASSEMBLED_DOMAIN_LIMIT_V46
    } else {
        LIMIT
    }
}

fn remaining_assembled_domain_bytes(
    map_total_limit: u64,
    map_total: u64,
    error: &'static str,
) -> Result<usize, &'static str> {
    usize::try_from(map_total_limit.checked_sub(map_total).ok_or(error)?).map_err(|_| error)
}

fn assembled_output_allowance(
    map_total_limit: u64,
    map_total: u64,
    proof_bytes: usize,
    error: &'static str,
) -> Result<usize, &'static str> {
    remaining_assembled_domain_bytes(map_total_limit, map_total, error)?
        .checked_sub(proof_bytes)
        .ok_or(error)
}

fn next_parent_payload_bytes(
    current: usize,
    payload_bytes: usize,
    map_total_limit: u64,
    error: &'static str,
) -> Result<usize, &'static str> {
    let next = current.checked_add(payload_bytes).ok_or(error)?;
    if u64::try_from(next).map_err(|_| error)? > map_total_limit {
        return Err(error);
    }
    Ok(next)
}

fn next_parent_payload_capacity(
    current: usize,
    payload_capacity: usize,
    map_total_limit: usize,
    error: &'static str,
) -> Result<usize, &'static str> {
    if payload_capacity > map_total_limit {
        return Err(error);
    }
    let next = current.checked_add(payload_capacity).ok_or(error)?;
    if next > map_total_limit {
        return Err(error);
    }
    Ok(next)
}

fn add_assembled_domain_bytes(
    current: u64,
    additional: usize,
    error: &'static str,
) -> Result<u64, &'static str> {
    current
        .checked_add(u64::try_from(additional).map_err(|_| error)?)
        .ok_or(error)
}

fn next_assembled_domain_total(
    current: u64,
    map_bytes: u64,
    map_total_limit: u64,
    error: &'static str,
) -> Result<u64, &'static str> {
    let next = current.checked_add(map_bytes).ok_or(error)?;
    if next > map_total_limit {
        return Err(error);
    }
    Ok(next)
}
const MODEL_SCOPE_PROFILE: &str = "placement-only-v1";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[allow(clippy::enum_variant_names)]
pub(crate) enum ModelScope {
    PlacementOnly,
    PlacementAndGameObject,
    PlacementAndGameObjectMissingRoot,
    PlacementAndGameObjectRootAndEncryptedUnavailable,
}
impl ModelScope {
    const fn as_str(self) -> &'static str {
        match self {
            Self::PlacementOnly => "placement_only",
            Self::PlacementAndGameObject => GAMEOBJECT_MODEL_SCOPE_PROFILE,
            Self::PlacementAndGameObjectMissingRoot => GAMEOBJECT_MISSING_ROOT_MODEL_SCOPE_PROFILE,
            Self::PlacementAndGameObjectRootAndEncryptedUnavailable => {
                GAMEOBJECT_ROOT_ENCRYPTED_UNAVAILABLE_MODEL_SCOPE_PROFILE
            }
        }
    }
    const fn profile(self) -> &'static str {
        match self {
            Self::PlacementOnly => MODEL_SCOPE_PROFILE,
            Self::PlacementAndGameObject => GAMEOBJECT_MODEL_SCOPE_PROFILE,
            Self::PlacementAndGameObjectMissingRoot => GAMEOBJECT_MISSING_ROOT_MODEL_SCOPE_PROFILE,
            Self::PlacementAndGameObjectRootAndEncryptedUnavailable => {
                GAMEOBJECT_ROOT_ENCRYPTED_UNAVAILABLE_MODEL_SCOPE_PROFILE
            }
        }
    }
}

#[cfg(test)]
mod tests;

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct ArtifactFact {
    pub path: String,
    pub bytes: u64,
    pub sha: [u8; 32],
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct SectionFact {
    pub ordinal: u32,
    pub records: u32,
    pub copies: u32,
    pub admitted: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct AvailableMapFact {
    pub file_data_id: u32,
    pub declared_bytes: u64,
    pub available_bytes: u64,
    pub full_ckey_verified: bool,
    pub encoded_key_authenticated: bool,
    pub declared_direct_rows: u64,
    pub declared_copy_records: u64,
    pub direct_rows: u64,
    pub copy_records: u64,
    pub materialized_rows: u64,
    pub sections: Vec<SectionFact>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct LiquidFact {
    pub file_data_id: u32,
    pub bytes: u64,
    pub magic: [u8; 4],
    pub direct_rows: u64,
    pub copy_records: u64,
    pub materialized_rows: u64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct DomainMapFact {
    pub id: u32,
    pub parent: Option<u32>,
    pub wdt_file_data_id: u32,
    pub depth: u32,
    pub is_parent: bool,
    pub root_absent: bool,
}

#[derive(Clone, Copy)]
struct GeometryPolicy {
    selection: MapSelection,
    missing_wdt_policy: run::MissingWdtPolicy,
    missing_m2_policy: run::MissingModelPolicy,
    missing_wmo_policy: run::MissingWmoPolicy,
    resource_profile: resources::Profile,
    allow_gameobject_missing_root: bool,
    allow_gameobject_encrypted_unavailable: bool,
}

impl GeometryPolicy {
    const fn new(
        selection: MapSelection,
        missing_wdt_policy: run::MissingWdtPolicy,
        missing_m2_policy: run::MissingModelPolicy,
        resource_profile: resources::Profile,
    ) -> Self {
        Self::new_with_missing_wmo(
            selection,
            missing_wdt_policy,
            missing_m2_policy,
            run::MissingWmoPolicy::Reject,
            resource_profile,
        )
    }

    const fn new_with_empty_wmo(
        selection: MapSelection,
        missing_wdt_policy: run::MissingWdtPolicy,
        missing_m2_policy: run::MissingModelPolicy,
        resource_profile: resources::Profile,
    ) -> Self {
        Self::new_with_missing_wmo(
            selection,
            missing_wdt_policy,
            missing_m2_policy,
            run::MissingWmoPolicy::DeclareVacatedAfterRootAbsentM2OrEmpty,
            resource_profile,
        )
    }

    const fn new_with_unselected_wmo(
        selection: MapSelection,
        missing_wdt_policy: run::MissingWdtPolicy,
        missing_m2_policy: run::MissingModelPolicy,
        resource_profile: resources::Profile,
    ) -> Self {
        Self::new_with_missing_wmo(
            selection,
            missing_wdt_policy,
            missing_m2_policy,
            run::MissingWmoPolicy::DeclareVacatedAfterRootAbsentM2OrEmptyOrUnselected,
            resource_profile,
        )
    }

    const fn new_with_material_filtered_wmo(
        selection: MapSelection,
        missing_wdt_policy: run::MissingWdtPolicy,
        missing_m2_policy: run::MissingModelPolicy,
        resource_profile: resources::Profile,
    ) -> Self {
        Self::new_with_missing_wmo(
            selection,
            missing_wdt_policy,
            missing_m2_policy,
            run::MissingWmoPolicy::DeclareVacatedAfterRootAbsentM2OrEmptyOrUnselectedOrMaterialFiltered,
            resource_profile,
        )
    }

    const fn new_with_dependency_heavy_material_filtered_wmo(
        selection: MapSelection,
        missing_wdt_policy: run::MissingWdtPolicy,
        missing_m2_policy: run::MissingModelPolicy,
    ) -> Self {
        Self::new_with_material_filtered_wmo(
            selection,
            missing_wdt_policy,
            missing_m2_policy,
            resources::Profile::DependencyHeavyV1,
        )
    }

    const fn new_with_read_heavy_material_filtered_wmo(
        selection: MapSelection,
        missing_wdt_policy: run::MissingWdtPolicy,
        missing_m2_policy: run::MissingModelPolicy,
    ) -> Self {
        Self::new_with_material_filtered_wmo(
            selection,
            missing_wdt_policy,
            missing_m2_policy,
            resources::Profile::ReadHeavyV1,
        )
    }

    const fn new_with_model_heavy_material_filtered_wmo(
        selection: MapSelection,
        missing_wdt_policy: run::MissingWdtPolicy,
        missing_m2_policy: run::MissingModelPolicy,
    ) -> Self {
        Self::new_with_material_filtered_wmo(
            selection,
            missing_wdt_policy,
            missing_m2_policy,
            resources::Profile::ModelHeavyV1,
        )
    }

    const fn new_with_missing_wmo(
        selection: MapSelection,
        missing_wdt_policy: run::MissingWdtPolicy,
        missing_m2_policy: run::MissingModelPolicy,
        missing_wmo_policy: run::MissingWmoPolicy,
        resource_profile: resources::Profile,
    ) -> Self {
        Self {
            selection,
            missing_wdt_policy,
            missing_m2_policy,
            missing_wmo_policy,
            resource_profile,
            allow_gameobject_missing_root: false,
            allow_gameobject_encrypted_unavailable: false,
        }
    }
}

impl GeometryPolicy {
    const fn with_gameobject_missing_root(mut self) -> Self {
        self.allow_gameobject_missing_root = true;
        self
    }

    const fn with_gameobject_root_or_encrypted_unavailable(mut self) -> Self {
        self.allow_gameobject_missing_root = true;
        self.allow_gameobject_encrypted_unavailable = true;
        self
    }

    fn with_gameobject_absence_if(mut self, enabled: bool) -> Self {
        self.allow_gameobject_missing_root = enabled;
        self.allow_gameobject_encrypted_unavailable = enabled
            && self.resource_profile
                == resources::Profile::GameObjectRootAndEncryptedUnavailableExpandedV1;
        self
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct GameObjectSectionFact {
    pub ordinal: u32,
    pub records: u32,
    pub copies: u32,
    pub admitted: bool,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum GameObjectDisposition {
    ZeroFileDataId,
    Texture,
    Collision,
    M2NoCollision,
    WmoDoodadOnly,
    MissingRoot,
    EncryptedUnavailable,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct GameObjectOutcomeFact {
    pub ordinal: u64,
    pub display_id: u32,
    pub file_data_id: u32,
    pub disposition: GameObjectDisposition,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct GameObjectFacts {
    pub declared_bytes: u64,
    pub available_bytes: u64,
    pub full_ckey_verified: bool,
    pub encoded_key_authenticated: bool,
    pub declared_direct_rows: u64,
    pub declared_copy_records: u64,
    pub direct_rows: u64,
    pub copy_records: u64,
    pub materialized_rows: u64,
    pub candidate_requests: u64,
    pub direct_outcomes: u64,
    pub zero_fdids: u64,
    pub textures: u64,
    pub m2_no_collision: u64,
    pub wmo_doodad_only: u64,
    pub missing_root_rows: u64,
    pub missing_root_fdids: Vec<u32>,
    pub encrypted_unavailable_rows: u64,
    pub encrypted_unavailable_fdids: Vec<u32>,
    pub collision_rows: u64,
    pub list_rows: u64,
    pub list_m2_rows: u64,
    pub list_wmo_rows: u64,
    pub list_vertex_visits: u64,
    pub list_raw_visit_bytes: u64,
    pub temp_bytes: u64,
    pub dtree_bytes: u64,
    pub temp_sha: [u8; 32],
    pub dtree_sha: [u8; 32],
    pub work_strict_candidates: u64,
    pub work_partial_candidates: u64,
    pub work_partial_bytes: u64,
    pub sections: Vec<GameObjectSectionFact>,
    pub outcomes: Vec<GameObjectOutcomeFact>,
}

pub(crate) struct Coverage {
    pub context: Context,
    pub selection: MapSelection,
    pub missing_wdt_policy: run::MissingWdtPolicy,
    pub missing_m2_policy: run::MissingModelPolicy,
    pub missing_wmo_policy: run::MissingWmoPolicy,
    pub resource_profile: resources::Profile,
    /// Producer-reported ROOT-absent M2 doodads nested in WMO, never direct M2.
    pub missing_m2: Vec<u32>,
    /// Producer-reported WMO models vacated by declared ROOT-absent M2 doodads;
    /// this report does not independently establish ROOT absence.
    pub missing_wmo: Vec<u32>,
    /// Producer-reported intrinsically empty WMO roots admitted by the v7 policy.
    pub empty_wmo: Vec<u32>,
    /// Producer-reported WMOs not selected for extraction by the v8 policy.
    pub unselected_wmo: Vec<u32>,
    /// Producer-reported WMOs whose geometry was fully removed by the material filter.
    pub material_filtered_wmo: Vec<u32>,
    /// GODI FDIDs whose typed outcome proves raw ROOT absence.
    pub gameobject_missing_root: Vec<u32>,
    /// GODI FDIDs whose bodies were unavailable because they were encrypted.
    pub gameobject_encrypted_unavailable: Vec<u32>,
    pub gameobject_scope: geometry::GameObjectScope,
    pub gameobject_file_data_id: u32,
    pub gameobject_block_reason: geometry::GameObjectBlockReason,
    pub gameobject_request_made: bool,
    pub gameobject_startup_reads: u64,
    pub gameobjects: Option<GameObjectFacts>,
    pub model_scope: ModelScope,
    pub liquid_object_scope: geometry::LiquidObjectScope,
    pub liquid_object_request_made: bool,
    pub map: AvailableMapFact,
    pub liquids: [LiquidFact; 2],
    pub maps: Vec<DomainMapFact>,
    pub artifacts: Vec<ArtifactFact>,
}

fn coverage_version(policy: GeometryPolicy) -> Result<u32, Error> {
    let GeometryPolicy {
        selection,
        missing_wdt_policy,
        missing_m2_policy,
        missing_wmo_policy,
        resource_profile,
        allow_gameobject_missing_root,
        allow_gameobject_encrypted_unavailable,
    } = policy;
    if allow_gameobject_missing_root
        && allow_gameobject_encrypted_unavailable
        && selection == MapSelection::All
        && missing_wdt_policy == run::MissingWdtPolicy::DeclareRootAbsent
        && missing_m2_policy == run::MissingModelPolicy::DeclareRootAbsentWmoDoodadM2
        && missing_wmo_policy
            == run::MissingWmoPolicy::DeclareVacatedAfterRootAbsentM2OrEmptyOrUnselectedOrMaterialFiltered
        && resource_profile
            == resources::Profile::GameObjectRootAndEncryptedUnavailableExpandedV1
    {
        return Ok(GAMEOBJECT_ROOT_ENCRYPTED_UNAVAILABLE_COVERAGE_VERSION);
    }
    if allow_gameobject_missing_root
        && !allow_gameobject_encrypted_unavailable
        && selection == MapSelection::All
        && missing_wdt_policy == run::MissingWdtPolicy::DeclareRootAbsent
        && missing_m2_policy == run::MissingModelPolicy::DeclareRootAbsentWmoDoodadM2
        && missing_wmo_policy
            == run::MissingWmoPolicy::DeclareVacatedAfterRootAbsentM2OrEmptyOrUnselectedOrMaterialFiltered
        && resource_profile == resources::Profile::GameObjectMissingRootExpandedV1
    {
        return Ok(GAMEOBJECT_MISSING_ROOT_COVERAGE_VERSION);
    }
    if !allow_gameobject_missing_root
        && !allow_gameobject_encrypted_unavailable
        && selection == MapSelection::All
        && missing_wdt_policy == run::MissingWdtPolicy::DeclareRootAbsent
        && missing_m2_policy == run::MissingModelPolicy::DeclareRootAbsentWmoDoodadM2
        && missing_wmo_policy
            == run::MissingWmoPolicy::DeclareVacatedAfterRootAbsentM2OrEmptyOrUnselectedOrMaterialFiltered
        && resource_profile == resources::Profile::GameObjectExpandedV1
    {
        return Ok(GAMEOBJECT_EXPANDED_MATERIAL_FILTERED_WMO_COVERAGE_VERSION);
    }
    match (
        selection,
        missing_wdt_policy,
        missing_m2_policy,
        missing_wmo_policy,
        resource_profile,
    ) {
        (
            MapSelection::MapAndAncestors(_),
            run::MissingWdtPolicy::Reject,
            run::MissingModelPolicy::Reject,
            run::MissingWmoPolicy::Reject,
            resources::Profile::BoundedV1,
        ) => Ok(COVERAGE_VERSION),
        (
            MapSelection::All,
            run::MissingWdtPolicy::Reject,
            run::MissingModelPolicy::Reject,
            run::MissingWmoPolicy::Reject,
            resources::Profile::BoundedV1,
        ) => Ok(AVAILABLE_COVERAGE_VERSION),
        (
            MapSelection::All,
            run::MissingWdtPolicy::DeclareRootAbsent,
            run::MissingModelPolicy::Reject,
            run::MissingWmoPolicy::Reject,
            resources::Profile::BoundedV1,
        ) => Ok(ROOT_ABSENT_COVERAGE_VERSION),
        (
            MapSelection::All,
            run::MissingWdtPolicy::DeclareRootAbsent,
            run::MissingModelPolicy::DeclareRootAbsentWmoDoodadM2,
            run::MissingWmoPolicy::Reject,
            resources::Profile::BoundedV1,
        ) => Ok(MODEL_ROOT_ABSENT_COVERAGE_VERSION),
        (
            MapSelection::All,
            run::MissingWdtPolicy::DeclareRootAbsent,
            run::MissingModelPolicy::DeclareRootAbsentWmoDoodadM2,
            run::MissingWmoPolicy::Reject,
            resources::Profile::RequestHeavyV1,
        ) => Ok(REQUEST_CAPACITY_COVERAGE_VERSION),
        (
            MapSelection::All,
            run::MissingWdtPolicy::DeclareRootAbsent,
            run::MissingModelPolicy::DeclareRootAbsentWmoDoodadM2,
            run::MissingWmoPolicy::DeclareVacatedAfterRootAbsentM2,
            resources::Profile::RequestHeavyV1,
        ) => Ok(VACATED_WMO_COVERAGE_VERSION),
        (
            MapSelection::All,
            run::MissingWdtPolicy::DeclareRootAbsent,
            run::MissingModelPolicy::DeclareRootAbsentWmoDoodadM2,
            run::MissingWmoPolicy::DeclareVacatedAfterRootAbsentM2OrEmpty,
            resources::Profile::RequestHeavyV1,
        ) => Ok(EMPTY_WMO_COVERAGE_VERSION),
        (
            MapSelection::All,
            run::MissingWdtPolicy::DeclareRootAbsent,
            run::MissingModelPolicy::DeclareRootAbsentWmoDoodadM2,
            run::MissingWmoPolicy::DeclareVacatedAfterRootAbsentM2OrEmptyOrUnselected,
            resources::Profile::RequestHeavyV1,
        ) => Ok(UNSELECTED_WMO_COVERAGE_VERSION),
        (
            MapSelection::All,
            run::MissingWdtPolicy::DeclareRootAbsent,
            run::MissingModelPolicy::DeclareRootAbsentWmoDoodadM2,
            run::MissingWmoPolicy::DeclareVacatedAfterRootAbsentM2OrEmptyOrUnselectedOrMaterialFiltered,
            resources::Profile::RequestHeavyV1,
        ) => Ok(MATERIAL_FILTERED_WMO_COVERAGE_VERSION),
        (
            MapSelection::All,
            run::MissingWdtPolicy::DeclareRootAbsent,
            run::MissingModelPolicy::DeclareRootAbsentWmoDoodadM2,
            run::MissingWmoPolicy::DeclareVacatedAfterRootAbsentM2OrEmptyOrUnselectedOrMaterialFiltered,
            resources::Profile::DependencyHeavyV1,
        ) => Ok(DEPENDENCY_HEAVY_MATERIAL_FILTERED_WMO_COVERAGE_VERSION),
        (
            MapSelection::All,
            run::MissingWdtPolicy::DeclareRootAbsent,
            run::MissingModelPolicy::DeclareRootAbsentWmoDoodadM2,
            run::MissingWmoPolicy::DeclareVacatedAfterRootAbsentM2OrEmptyOrUnselectedOrMaterialFiltered,
            resources::Profile::SourceHeavyV1,
        ) => Ok(SOURCE_HEAVY_MATERIAL_FILTERED_WMO_COVERAGE_VERSION),
        (
            MapSelection::All,
            run::MissingWdtPolicy::DeclareRootAbsent,
            run::MissingModelPolicy::DeclareRootAbsentWmoDoodadM2,
            run::MissingWmoPolicy::DeclareVacatedAfterRootAbsentM2OrEmptyOrUnselectedOrMaterialFiltered,
            resources::Profile::ReadHeavyV1,
        ) => Ok(READ_HEAVY_MATERIAL_FILTERED_WMO_COVERAGE_VERSION),
        (
            MapSelection::All,
            run::MissingWdtPolicy::DeclareRootAbsent,
            run::MissingModelPolicy::DeclareRootAbsentWmoDoodadM2,
            run::MissingWmoPolicy::DeclareVacatedAfterRootAbsentM2OrEmptyOrUnselectedOrMaterialFiltered,
            resources::Profile::ModelHeavyV1,
        ) => Ok(MODEL_HEAVY_MATERIAL_FILTERED_WMO_COVERAGE_VERSION),
        (
            MapSelection::All,
            run::MissingWdtPolicy::DeclareRootAbsent,
            run::MissingModelPolicy::DeclareRootAbsentWmoDoodadM2,
            run::MissingWmoPolicy::DeclareVacatedAfterRootAbsentM2OrEmptyOrUnselectedOrMaterialFiltered,
            resources::Profile::SourceExpandedV1,
        ) => Ok(SOURCE_EXPANDED_MATERIAL_FILTERED_WMO_COVERAGE_VERSION),
        (
            MapSelection::All,
            run::MissingWdtPolicy::DeclareRootAbsent,
            run::MissingModelPolicy::DeclareRootAbsentWmoDoodadM2,
            run::MissingWmoPolicy::DeclareVacatedAfterRootAbsentM2OrEmptyOrUnselectedOrMaterialFiltered,
            resources::Profile::ModelOutputExpandedV1,
        ) => Ok(MODEL_OUTPUT_EXPANDED_MATERIAL_FILTERED_WMO_COVERAGE_VERSION),
        (
            MapSelection::All,
            run::MissingWdtPolicy::DeclareRootAbsent,
            run::MissingModelPolicy::DeclareRootAbsentWmoDoodadM2,
            run::MissingWmoPolicy::DeclareVacatedAfterRootAbsentM2OrEmptyOrUnselectedOrMaterialFiltered,
            resources::Profile::ModelBytesExpandedV1,
        ) => Ok(MODEL_BYTES_EXPANDED_MATERIAL_FILTERED_WMO_COVERAGE_VERSION),
        (
            MapSelection::All,
            run::MissingWdtPolicy::DeclareRootAbsent,
            run::MissingModelPolicy::DeclareRootAbsentWmoDoodadM2,
            run::MissingWmoPolicy::DeclareVacatedAfterRootAbsentM2OrEmptyOrUnselectedOrMaterialFiltered,
            resources::Profile::SourceExpandedV2,
        ) => Ok(SOURCE_EXPANDED_V2_MATERIAL_FILTERED_WMO_COVERAGE_VERSION),
        (
            MapSelection::All,
            run::MissingWdtPolicy::DeclareRootAbsent,
            run::MissingModelPolicy::DeclareRootAbsentWmoDoodadM2,
            run::MissingWmoPolicy::DeclareVacatedAfterRootAbsentM2OrEmptyOrUnselectedOrMaterialFiltered,
            resources::Profile::PlacementExpandedV1,
        ) => Ok(PLACEMENT_EXPANDED_MATERIAL_FILTERED_WMO_COVERAGE_VERSION),
        (
            MapSelection::All,
            run::MissingWdtPolicy::DeclareRootAbsent,
            run::MissingModelPolicy::DeclareRootAbsentWmoDoodadM2,
            run::MissingWmoPolicy::DeclareVacatedAfterRootAbsentM2OrEmptyOrUnselectedOrMaterialFiltered,
            resources::Profile::AssemblyExpandedV1,
        ) => Ok(ASSEMBLY_EXPANDED_MATERIAL_FILTERED_WMO_COVERAGE_VERSION),
        (
            MapSelection::All,
            run::MissingWdtPolicy::DeclareRootAbsent,
            run::MissingModelPolicy::DeclareRootAbsentWmoDoodadM2,
            run::MissingWmoPolicy::DeclareVacatedAfterRootAbsentM2OrEmptyOrUnselectedOrMaterialFiltered,
            resources::Profile::GameObjectExpandedV1,
        ) if !allow_gameobject_missing_root => {
            Ok(GAMEOBJECT_EXPANDED_MATERIAL_FILTERED_WMO_COVERAGE_VERSION)
        }
        (
            MapSelection::All,
            run::MissingWdtPolicy::DeclareRootAbsent,
            run::MissingModelPolicy::DeclareRootAbsentWmoDoodadM2,
            run::MissingWmoPolicy::DeclareVacatedAfterRootAbsentM2OrEmptyOrUnselectedOrMaterialFiltered,
            resources::Profile::GameObjectMissingRootExpandedV1,
        ) if allow_gameobject_missing_root && !allow_gameobject_encrypted_unavailable => {
            Ok(GAMEOBJECT_MISSING_ROOT_COVERAGE_VERSION)
        },
        (
            MapSelection::All,
            run::MissingWdtPolicy::DeclareRootAbsent,
            run::MissingModelPolicy::DeclareRootAbsentWmoDoodadM2,
            run::MissingWmoPolicy::DeclareVacatedAfterRootAbsentM2OrEmptyOrUnselectedOrMaterialFiltered,
            resources::Profile::GameObjectRootAndEncryptedUnavailableExpandedV1,
        ) if allow_gameobject_missing_root && allow_gameobject_encrypted_unavailable => {
            Ok(GAMEOBJECT_ROOT_ENCRYPTED_UNAVAILABLE_COVERAGE_VERSION)
        },
        _ => Err(SCHEMA.into()),
    }
}

fn coverage_path(policy: GeometryPolicy) -> Result<&'static str, Error> {
    match coverage_version(policy)? {
        COVERAGE_VERSION => Ok(COVERAGE_PATH),
        AVAILABLE_COVERAGE_VERSION => Ok(AVAILABLE_COVERAGE_PATH),
        ROOT_ABSENT_COVERAGE_VERSION => Ok(ROOT_ABSENT_COVERAGE_PATH),
        MODEL_ROOT_ABSENT_COVERAGE_VERSION => Ok(MODEL_ROOT_ABSENT_COVERAGE_PATH),
        REQUEST_CAPACITY_COVERAGE_VERSION => Ok(REQUEST_CAPACITY_COVERAGE_PATH),
        VACATED_WMO_COVERAGE_VERSION => Ok(VACATED_WMO_COVERAGE_PATH),
        EMPTY_WMO_COVERAGE_VERSION => Ok(EMPTY_WMO_COVERAGE_PATH),
        UNSELECTED_WMO_COVERAGE_VERSION => Ok(UNSELECTED_WMO_COVERAGE_PATH),
        MATERIAL_FILTERED_WMO_COVERAGE_VERSION => Ok(MATERIAL_FILTERED_WMO_COVERAGE_PATH),
        DEPENDENCY_HEAVY_MATERIAL_FILTERED_WMO_COVERAGE_VERSION => {
            Ok(DEPENDENCY_HEAVY_MATERIAL_FILTERED_WMO_COVERAGE_PATH)
        }
        SOURCE_HEAVY_MATERIAL_FILTERED_WMO_COVERAGE_VERSION => {
            Ok(SOURCE_HEAVY_MATERIAL_FILTERED_WMO_COVERAGE_PATH)
        }
        READ_HEAVY_MATERIAL_FILTERED_WMO_COVERAGE_VERSION => {
            Ok(READ_HEAVY_MATERIAL_FILTERED_WMO_COVERAGE_PATH)
        }
        MODEL_HEAVY_MATERIAL_FILTERED_WMO_COVERAGE_VERSION => {
            Ok(MODEL_HEAVY_MATERIAL_FILTERED_WMO_COVERAGE_PATH)
        }
        SOURCE_EXPANDED_MATERIAL_FILTERED_WMO_COVERAGE_VERSION => {
            Ok(SOURCE_EXPANDED_MATERIAL_FILTERED_WMO_COVERAGE_PATH)
        }
        MODEL_OUTPUT_EXPANDED_MATERIAL_FILTERED_WMO_COVERAGE_VERSION => {
            Ok(MODEL_OUTPUT_EXPANDED_MATERIAL_FILTERED_WMO_COVERAGE_PATH)
        }
        MODEL_BYTES_EXPANDED_MATERIAL_FILTERED_WMO_COVERAGE_VERSION => {
            Ok(MODEL_BYTES_EXPANDED_MATERIAL_FILTERED_WMO_COVERAGE_PATH)
        }
        SOURCE_EXPANDED_V2_MATERIAL_FILTERED_WMO_COVERAGE_VERSION => {
            Ok(SOURCE_EXPANDED_V2_MATERIAL_FILTERED_WMO_COVERAGE_PATH)
        }
        PLACEMENT_EXPANDED_MATERIAL_FILTERED_WMO_COVERAGE_VERSION => {
            Ok(PLACEMENT_EXPANDED_MATERIAL_FILTERED_WMO_COVERAGE_PATH)
        }
        ASSEMBLY_EXPANDED_MATERIAL_FILTERED_WMO_COVERAGE_VERSION => {
            Ok(ASSEMBLY_EXPANDED_MATERIAL_FILTERED_WMO_COVERAGE_PATH)
        }
        GAMEOBJECT_EXPANDED_MATERIAL_FILTERED_WMO_COVERAGE_VERSION => {
            Ok(GAMEOBJECT_EXPANDED_MATERIAL_FILTERED_WMO_COVERAGE_PATH)
        }
        GAMEOBJECT_MISSING_ROOT_COVERAGE_VERSION => {
            Ok(GAMEOBJECT_MISSING_ROOT_MATERIAL_FILTERED_WMO_COVERAGE_PATH)
        }
        GAMEOBJECT_ROOT_ENCRYPTED_UNAVAILABLE_COVERAGE_VERSION => {
            Ok(GAMEOBJECT_ROOT_ENCRYPTED_UNAVAILABLE_MATERIAL_FILTERED_WMO_COVERAGE_PATH)
        }
        _ => Err(SCHEMA.into()),
    }
}

pub(crate) fn m2_file_name(id: u32) -> String {
    format!("FILE{id:08X}.xxx")
}

fn local_model_limit(profile: resources::Profile) -> (usize, &'static str) {
    if matches!(
        profile,
        resources::Profile::ModelOutputExpandedV1
            | resources::Profile::ModelBytesExpandedV1
            | resources::Profile::SourceExpandedV2
            | resources::Profile::PlacementExpandedV1
            | resources::Profile::AssemblyExpandedV1
            | resources::Profile::GameObjectExpandedV1
            | resources::Profile::GameObjectMissingRootExpandedV1
            | resources::Profile::GameObjectRootAndEncryptedUnavailableExpandedV1
    ) {
        (profile.collector_limits().max_models, MODEL_COUNT_LIMIT)
    } else {
        (4096, LIMIT)
    }
}

fn admit_model_count(
    current: usize,
    max_models: usize,
    limit: &'static str,
) -> Result<(), &'static str> {
    if current >= max_models {
        Err(limit)
    } else {
        Ok(())
    }
}

pub(crate) fn validate_missing_wmo(
    missing_wmo: &[u32],
    missing_wmo_policy: run::MissingWmoPolicy,
    missing_m2: &[u32],
) -> Result<(), Error> {
    if missing_wmo.len() > COVERAGE_MAX_MISSING_WMO {
        return Err(LIMIT.into());
    }
    if missing_wmo_policy == run::MissingWmoPolicy::Reject && !missing_wmo.is_empty() {
        return Err(SCHEMA.into());
    }
    let mut previous = None;
    for &id in missing_wmo {
        if id == 0
            || previous.is_some_and(|previous| previous >= id)
            || missing_m2.binary_search(&id).is_ok()
        {
            return Err(SCHEMA.into());
        }
        previous = Some(id);
    }
    Ok(())
}

pub(crate) fn validate_empty_wmo(
    empty_wmo: &[u32],
    missing_wmo_policy: run::MissingWmoPolicy,
    missing_wmo: &[u32],
    missing_m2: &[u32],
) -> Result<(), Error> {
    if empty_wmo.len() > COVERAGE_MAX_EMPTY_WMO {
        return Err(LIMIT.into());
    }
    if !matches!(
        missing_wmo_policy,
        run::MissingWmoPolicy::DeclareVacatedAfterRootAbsentM2OrEmpty
            | run::MissingWmoPolicy::DeclareVacatedAfterRootAbsentM2OrEmptyOrUnselected
            | run::MissingWmoPolicy::DeclareVacatedAfterRootAbsentM2OrEmptyOrUnselectedOrMaterialFiltered
    ) && !empty_wmo.is_empty()
    {
        return Err(SCHEMA.into());
    }
    let mut previous = None;
    for &id in empty_wmo {
        if id == 0
            || previous.is_some_and(|previous| previous >= id)
            || missing_wmo.binary_search(&id).is_ok()
            || missing_m2.binary_search(&id).is_ok()
        {
            return Err(SCHEMA.into());
        }
        previous = Some(id);
    }
    Ok(())
}

pub(crate) fn validate_unselected_wmo(
    unselected_wmo: &[u32],
    missing_wmo_policy: run::MissingWmoPolicy,
    missing_m2: &[u32],
    missing_wmo: &[u32],
    empty_wmo: &[u32],
) -> Result<(), Error> {
    if unselected_wmo.len() > COVERAGE_MAX_UNSELECTED_WMO {
        return Err(LIMIT.into());
    }
    if !matches!(
        missing_wmo_policy,
        run::MissingWmoPolicy::DeclareVacatedAfterRootAbsentM2OrEmptyOrUnselected
            | run::MissingWmoPolicy::DeclareVacatedAfterRootAbsentM2OrEmptyOrUnselectedOrMaterialFiltered
    ) && !unselected_wmo.is_empty()
    {
        return Err(SCHEMA.into());
    }
    let mut previous = None;
    for &id in unselected_wmo {
        if id == 0
            || previous.is_some_and(|previous| previous >= id)
            || missing_m2.binary_search(&id).is_ok()
            || missing_wmo.binary_search(&id).is_ok()
            || empty_wmo.binary_search(&id).is_ok()
        {
            return Err(SCHEMA.into());
        }
        previous = Some(id);
    }
    Ok(())
}

pub(crate) fn validate_material_filtered_wmo(
    material_filtered_wmo: &[u32],
    missing_wmo_policy: run::MissingWmoPolicy,
    missing_m2: &[u32],
    missing_wmo: &[u32],
    empty_wmo: &[u32],
    unselected_wmo: &[u32],
) -> Result<(), Error> {
    if material_filtered_wmo.len() > COVERAGE_MAX_MATERIAL_FILTERED_WMO {
        return Err(LIMIT.into());
    }
    if !matches!(
        missing_wmo_policy,
        run::MissingWmoPolicy::DeclareVacatedAfterRootAbsentM2OrEmptyOrUnselectedOrMaterialFiltered
    ) && !material_filtered_wmo.is_empty()
    {
        return Err(SCHEMA.into());
    }
    let mut previous = None;
    for &id in material_filtered_wmo {
        if id == 0
            || previous.is_some_and(|previous| previous >= id)
            || missing_m2.binary_search(&id).is_ok()
            || missing_wmo.binary_search(&id).is_ok()
            || empty_wmo.binary_search(&id).is_ok()
            || unselected_wmo.binary_search(&id).is_ok()
        {
            return Err(SCHEMA.into());
        }
        previous = Some(id);
    }
    Ok(())
}

fn wmo_output_name(id: u32) -> String {
    m2_file_name(id)
}

fn model_output_file_data_id(name: &str) -> Option<u32> {
    let bytes = name.as_bytes();
    if bytes.len() != 16 || &bytes[..4] != b"FILE" || &bytes[12..] != b".xxx" {
        return None;
    }
    let id = u32::from_str_radix(str::from_utf8(&bytes[4..12]).ok()?, 16).ok()?;
    (id != 0 && wmo_output_name(id) == name).then_some(id)
}

fn is_omitted_wmo_name(
    name: &str,
    missing_m2: &[u32],
    missing_wmo: &[u32],
    empty_wmo: &[u32],
    unselected_wmo: &[u32],
    material_filtered_wmo: &[u32],
    include_missing_m2: bool,
) -> bool {
    model_output_file_data_id(name).is_some_and(|id| {
        (include_missing_m2 && missing_m2.binary_search(&id).is_ok())
            || missing_wmo.binary_search(&id).is_ok()
            || empty_wmo.binary_search(&id).is_ok()
            || unselected_wmo.binary_search(&id).is_ok()
            || material_filtered_wmo.binary_search(&id).is_ok()
    })
}

fn validate_wmo_omission_artifacts(
    omitted_wmo: &[u32],
    used_models: &BTreeMap<String, ()>,
    artifact_inventory: &BTreeMap<String, ArtifactFact>,
) -> Result<(), Error> {
    for &id in omitted_wmo {
        let name = wmo_output_name(id);
        if used_models.contains_key(&name)
            || artifact_inventory.contains_key(&format!("vmaps/{name}.vmo"))
        {
            return Err(SCHEMA.into());
        }
    }
    Ok(())
}

pub(crate) fn validate_missing_m2(
    missing_m2: &[u32],
    missing_m2_policy: run::MissingModelPolicy,
) -> Result<(), Error> {
    if missing_m2.len() > COVERAGE_MAX_MISSING_M2 {
        return Err(LIMIT.into());
    }
    if missing_m2_policy == run::MissingModelPolicy::Reject && !missing_m2.is_empty() {
        return Err(SCHEMA.into());
    }
    let mut previous = None;
    for &id in missing_m2 {
        if id == 0 || previous.is_some_and(|previous| previous >= id) {
            return Err(SCHEMA.into());
        }
        previous = Some(id);
    }
    Ok(())
}

fn validate_missing_model_artifacts(
    missing_m2: &[u32],
    used_models: &BTreeMap<String, ()>,
    artifact_inventory: &BTreeMap<String, ArtifactFact>,
) -> Result<(), Error> {
    for &id in missing_m2 {
        let name = m2_file_name(id);
        if used_models.contains_key(&name)
            || artifact_inventory.contains_key(&format!("vmaps/{name}.vmo"))
        {
            return Err(SCHEMA.into());
        }
    }
    Ok(())
}

fn completed_domain(
    completed: &geometry::Completed,
    policy: GeometryPolicy,
) -> Result<Vec<DomainMapFact>, Error> {
    coverage_version(policy)?;
    let GeometryPolicy {
        selection,
        missing_wdt_policy,
        ..
    } = policy;
    let root_absent = completed.world().root_absent();
    if root_absent.len() > COVERAGE_MAX_MAPS
        || (missing_wdt_policy == run::MissingWdtPolicy::Reject && !root_absent.is_empty())
    {
        return Err(SCHEMA.into());
    }
    let topology = completed.world().topology();
    let domain_count = topology.selected_active().count();
    if root_absent.len() > domain_count {
        return Err(SCHEMA.into());
    }
    let mut declarations = BTreeMap::new();
    for fact in root_absent {
        if declarations.insert(fact.map_id, *fact).is_some() {
            return Err(SCHEMA.into());
        }
    }
    let mut maps = Vec::new();
    maps.try_reserve_exact(domain_count)
        .map_err(|_| Error::from("product_allocation"))?;
    for entry in topology.selected_active() {
        let omission = declarations.remove(&entry.id());
        if omission.is_some_and(|fact| fact.wdt_file_data_id != entry.wdt_file_data_id()) {
            return Err(SCHEMA.into());
        }
        maps.push(DomainMapFact {
            id: entry.id(),
            parent: entry.parent_map_id(),
            wdt_file_data_id: entry.wdt_file_data_id(),
            depth: u32::try_from(entry.depth()).map_err(|_| LIMIT)?,
            is_parent: entry.is_parent(),
            root_absent: omission.is_some(),
        });
    }
    if !declarations.is_empty() {
        return Err(SCHEMA.into());
    }
    maps.sort_unstable_by_key(|entry| entry.id);
    if maps.len() > COVERAGE_MAX_MAPS
        || maps
            .windows(2)
            .any(|entries| entries[0].id == entries[1].id)
    {
        return Err(SCHEMA.into());
    }
    match selection {
        MapSelection::MapAndAncestors(id) => {
            if maps.is_empty() || !maps.iter().any(|entry| entry.id == id) {
                return Err(SCHEMA.into());
            }
        }
        MapSelection::All => {
            if maps.len() != completed.world().map_stats().active_maps {
                return Err(SCHEMA.into());
            }
        }
    }
    Ok(maps)
}

fn validate_omitted_artifacts(
    maps: &[DomainMapFact],
    artifact_inventory: &BTreeMap<String, ArtifactFact>,
) -> Result<(), Error> {
    for map in maps.iter().filter(|map| map.root_absent) {
        let (tree_path, proof_path) = map_paths(map.id);
        let prefix = format!("vmaps/{:04}/", map.id);
        if artifact_inventory.contains_key(&tree_path)
            || artifact_inventory.contains_key(&proof_path)
            || artifact_inventory
                .range(prefix.clone()..)
                .take_while(|(path, _)| path.starts_with(&prefix))
                .next()
                .is_some()
        {
            return Err(SCHEMA.into());
        }
    }
    Ok(())
}

fn gameobject_facts_from_geometry(
    facts: &geometry::GameObjectFacts,
) -> Result<GameObjectFacts, Error> {
    let mut sections = Vec::new();
    sections
        .try_reserve_exact(facts.sections.len())
        .map_err(|_| Error::from("product_allocation"))?;
    sections.extend(facts.sections.iter().map(|section| GameObjectSectionFact {
        ordinal: section.ordinal,
        records: section.declared_records,
        copies: section.declared_copies,
        admitted: section.admitted,
    }));
    let mut outcomes = Vec::new();
    outcomes
        .try_reserve_exact(facts.outcomes.len())
        .map_err(|_| Error::from("product_allocation"))?;
    outcomes.extend(facts.outcomes.iter().map(|outcome| GameObjectOutcomeFact {
        ordinal: outcome.ordinal,
        display_id: outcome.display_id,
        file_data_id: outcome.file_data_id,
        disposition: match outcome.outcome {
            forever_casc::collision::available_70374::GameObjectOutcome::ZeroFileDataId => {
                GameObjectDisposition::ZeroFileDataId
            }
            forever_casc::collision::available_70374::GameObjectOutcome::Texture => {
                GameObjectDisposition::Texture
            }
            forever_casc::collision::available_70374::GameObjectOutcome::Collision => {
                GameObjectDisposition::Collision
            }
            forever_casc::collision::available_70374::GameObjectOutcome::M2NoCollision => {
                GameObjectDisposition::M2NoCollision
            }
            forever_casc::collision::available_70374::GameObjectOutcome::WmoDoodadOnly => {
                GameObjectDisposition::WmoDoodadOnly
            }
            forever_casc::collision::available_70374::GameObjectOutcome::MissingRoot => {
                GameObjectDisposition::MissingRoot
            }
            forever_casc::collision::available_70374::GameObjectOutcome::EncryptedUnavailable => {
                GameObjectDisposition::EncryptedUnavailable
            }
        },
    }));
    Ok(GameObjectFacts {
        declared_bytes: facts.declared_bytes,
        available_bytes: facts.available_bytes,
        full_ckey_verified: facts.full_ckey_verified,
        encoded_key_authenticated: facts.encoded_key_authenticated,
        declared_direct_rows: facts.declared_direct_rows,
        declared_copy_records: facts.declared_copy_records,
        direct_rows: facts.direct_rows,
        copy_records: facts.copy_records,
        materialized_rows: facts.materialized_rows,
        candidate_requests: facts.candidate_requests,
        direct_outcomes: facts.direct_outcomes,
        zero_fdids: facts.zero_fdids,
        textures: facts.textures,
        m2_no_collision: facts.m2_no_collision,
        wmo_doodad_only: facts.wmo_doodad_only,
        missing_root_rows: facts.missing_root_rows,
        missing_root_fdids: facts.missing_root_fdids.clone(),
        encrypted_unavailable_rows: facts.encrypted_unavailable_rows,
        encrypted_unavailable_fdids: facts.encrypted_unavailable_fdids.clone(),
        collision_rows: facts.collision_rows,
        list_rows: facts.list_rows,
        list_m2_rows: facts.list_m2_rows,
        list_wmo_rows: facts.list_wmo_rows,
        list_vertex_visits: facts.list_vertex_visits,
        list_raw_visit_bytes: facts.list_raw_visit_bytes,
        temp_bytes: facts.temp_bytes,
        dtree_bytes: facts.dtree_bytes,
        temp_sha: facts.temp_sha256,
        dtree_sha: facts.dtree_sha256,
        work_strict_candidates: u64::try_from(facts.work.strict_candidate_attempts)
            .map_err(|_| LIMIT)?,
        work_partial_candidates: u64::try_from(facts.work.partial_candidate_attempts)
            .map_err(|_| LIMIT)?,
        work_partial_bytes: facts.work.partial_encoded_bytes_read,
        sections,
        outcomes,
    })
}

fn derive_model_scope(
    completed: &geometry::Completed,
    used_models: &std::collections::BTreeMap<String, ()>,
    artifact_inventory: &BTreeMap<String, ArtifactFact>,
) -> Result<(ModelScope, u64), Error> {
    let startup = completed.world().startup().data();
    let temp_list = forever_vmap::gameobject_models::TempList::parse(
        startup.lists().temp_bytes(),
        forever_vmap::gameobject_models::Limits::default(),
    )
    .map_err(|_| Error::from(SCHEMA))?;
    let startup_reads = u64::try_from(startup.startup_facts().reads).map_err(|_| LIMIT)?;
    if startup.table().file_data_id() != 0
        || startup_reads != 0
        || startup.stats().direct_rows != 0
        || startup.stats().copy_records != 0
        || startup.stats().candidate_requests != 0
        || !startup.outcomes().is_empty()
        || startup.lists().stats().rows != 0
        || temp_list.rows().next().is_some()
    {
        return Err(SCHEMA.into());
    }

    // Coverage may name only model artifacts corresponding one-to-one with the
    // actual placement requests already recorded by this producer. The BTreeMap
    // order makes this a bounded, allocation-free comparison.
    let mut artifacts = artifact_inventory
        .keys()
        .filter(|path| path.ends_with(".vmo"));
    for name in used_models.keys() {
        let path = artifacts.next().ok_or(SCHEMA)?;
        let artifact_name = path
            .strip_prefix("vmaps/")
            .and_then(|path| path.strip_suffix(".vmo"))
            .filter(|name| !name.is_empty() && !name.contains('/'))
            .ok_or(SCHEMA)?;
        if artifact_name != name {
            return Err(SCHEMA.into());
        }
    }
    if artifacts.next().is_some() {
        return Err(SCHEMA.into());
    }
    Ok((ModelScope::PlacementOnly, startup_reads))
}

impl Coverage {
    fn from_completed(
        completed: &geometry::Completed,
        context: Context,
        policy: GeometryPolicy,
        gameobject_file_data_id: u32,
        used_models: &std::collections::BTreeMap<String, ()>,
        artifact_inventory: &BTreeMap<String, ArtifactFact>,
    ) -> Result<Self, Error> {
        let GeometryPolicy {
            selection,
            missing_wdt_policy,
            missing_m2_policy,
            missing_wmo_policy,
            resource_profile,
            allow_gameobject_missing_root,
            allow_gameobject_encrypted_unavailable,
        } = policy;
        let expected_profile = if allow_gameobject_encrypted_unavailable {
            if !allow_gameobject_missing_root
                || selection != MapSelection::All
                || missing_wdt_policy != run::MissingWdtPolicy::DeclareRootAbsent
                || missing_m2_policy != run::MissingModelPolicy::DeclareRootAbsentWmoDoodadM2
                || missing_wmo_policy
                    != run::MissingWmoPolicy::DeclareVacatedAfterRootAbsentM2OrEmptyOrUnselectedOrMaterialFiltered
                || resource_profile
                    != resources::Profile::GameObjectRootAndEncryptedUnavailableExpandedV1
            {
                return Err(SCHEMA.into());
            }
            GAMEOBJECT_ROOT_ENCRYPTED_UNAVAILABLE_COMPLETION_PROFILE
        } else if allow_gameobject_missing_root {
            if selection != MapSelection::All
                || missing_wdt_policy != run::MissingWdtPolicy::DeclareRootAbsent
                || missing_m2_policy != run::MissingModelPolicy::DeclareRootAbsentWmoDoodadM2
                || missing_wmo_policy
                    != run::MissingWmoPolicy::DeclareVacatedAfterRootAbsentM2OrEmptyOrUnselectedOrMaterialFiltered
                || resource_profile != resources::Profile::GameObjectMissingRootExpandedV1
            {
                return Err(SCHEMA.into());
            }
            GAMEOBJECT_MISSING_ROOT_COMPLETION_PROFILE
        } else {
            match (
                selection,
                missing_wdt_policy,
                missing_m2_policy,
                missing_wmo_policy,
                resource_profile,
            ) {
            (
                MapSelection::MapAndAncestors(_),
                run::MissingWdtPolicy::Reject,
                run::MissingModelPolicy::Reject,
                run::MissingWmoPolicy::Reject,
                resources::Profile::BoundedV1,
            ) => geometry::PROFILE,
            (
                MapSelection::All,
                run::MissingWdtPolicy::Reject,
                run::MissingModelPolicy::Reject,
                run::MissingWmoPolicy::Reject,
                resources::Profile::BoundedV1,
            ) => geometry::AVAILABLE_PROFILE,
            (
                MapSelection::All,
                run::MissingWdtPolicy::DeclareRootAbsent,
                run::MissingModelPolicy::Reject,
                run::MissingWmoPolicy::Reject,
                resources::Profile::BoundedV1,
            ) => geometry::AVAILABLE_ROOT_ABSENT_PROFILE,
            (
                MapSelection::All,
                run::MissingWdtPolicy::DeclareRootAbsent,
                run::MissingModelPolicy::DeclareRootAbsentWmoDoodadM2,
                run::MissingWmoPolicy::Reject,
                resources::Profile::BoundedV1,
            ) => geometry::AVAILABLE_MODEL_ROOT_ABSENT_PROFILE,
            (
                MapSelection::All,
                run::MissingWdtPolicy::DeclareRootAbsent,
                run::MissingModelPolicy::DeclareRootAbsentWmoDoodadM2,
                run::MissingWmoPolicy::Reject,
                resources::Profile::RequestHeavyV1,
            ) => geometry::AVAILABLE_MODEL_ROOT_ABSENT_REQUEST_CAPACITY_PROFILE,
            (
                MapSelection::All,
                run::MissingWdtPolicy::DeclareRootAbsent,
                run::MissingModelPolicy::DeclareRootAbsentWmoDoodadM2,
                run::MissingWmoPolicy::DeclareVacatedAfterRootAbsentM2,
                resources::Profile::RequestHeavyV1,
            ) => VACATED_WMO_COMPLETION_PROFILE,
            (
                MapSelection::All,
                run::MissingWdtPolicy::DeclareRootAbsent,
                run::MissingModelPolicy::DeclareRootAbsentWmoDoodadM2,
                run::MissingWmoPolicy::DeclareVacatedAfterRootAbsentM2OrEmpty,
                resources::Profile::RequestHeavyV1,
            ) => EMPTY_WMO_COMPLETION_PROFILE,
            (
                MapSelection::All,
                run::MissingWdtPolicy::DeclareRootAbsent,
                run::MissingModelPolicy::DeclareRootAbsentWmoDoodadM2,
                run::MissingWmoPolicy::DeclareVacatedAfterRootAbsentM2OrEmptyOrUnselected,
                resources::Profile::RequestHeavyV1,
            ) => UNSELECTED_WMO_COMPLETION_PROFILE,
            (
                MapSelection::All,
                run::MissingWdtPolicy::DeclareRootAbsent,
                run::MissingModelPolicy::DeclareRootAbsentWmoDoodadM2,
                run::MissingWmoPolicy::DeclareVacatedAfterRootAbsentM2OrEmptyOrUnselectedOrMaterialFiltered,
                resources::Profile::RequestHeavyV1,
            ) => MATERIAL_FILTERED_WMO_COMPLETION_PROFILE,
            (
                MapSelection::All,
                run::MissingWdtPolicy::DeclareRootAbsent,
                run::MissingModelPolicy::DeclareRootAbsentWmoDoodadM2,
                run::MissingWmoPolicy::DeclareVacatedAfterRootAbsentM2OrEmptyOrUnselectedOrMaterialFiltered,
                resources::Profile::DependencyHeavyV1,
            ) => DEPENDENCY_HEAVY_MATERIAL_FILTERED_WMO_COMPLETION_PROFILE,
            (
                MapSelection::All,
                run::MissingWdtPolicy::DeclareRootAbsent,
                run::MissingModelPolicy::DeclareRootAbsentWmoDoodadM2,
                run::MissingWmoPolicy::DeclareVacatedAfterRootAbsentM2OrEmptyOrUnselectedOrMaterialFiltered,
                resources::Profile::SourceHeavyV1,
            ) => SOURCE_HEAVY_MATERIAL_FILTERED_WMO_COMPLETION_PROFILE,
            (
                MapSelection::All,
                run::MissingWdtPolicy::DeclareRootAbsent,
                run::MissingModelPolicy::DeclareRootAbsentWmoDoodadM2,
                run::MissingWmoPolicy::DeclareVacatedAfterRootAbsentM2OrEmptyOrUnselectedOrMaterialFiltered,
                resources::Profile::ReadHeavyV1,
            ) => READ_HEAVY_MATERIAL_FILTERED_WMO_COMPLETION_PROFILE,
            (
                MapSelection::All,
                run::MissingWdtPolicy::DeclareRootAbsent,
                run::MissingModelPolicy::DeclareRootAbsentWmoDoodadM2,
                run::MissingWmoPolicy::DeclareVacatedAfterRootAbsentM2OrEmptyOrUnselectedOrMaterialFiltered,
                resources::Profile::ModelHeavyV1,
            ) => MODEL_HEAVY_MATERIAL_FILTERED_WMO_COMPLETION_PROFILE,
            (
                MapSelection::All,
                run::MissingWdtPolicy::DeclareRootAbsent,
                run::MissingModelPolicy::DeclareRootAbsentWmoDoodadM2,
                run::MissingWmoPolicy::DeclareVacatedAfterRootAbsentM2OrEmptyOrUnselectedOrMaterialFiltered,
                resources::Profile::SourceExpandedV1,
            ) => SOURCE_EXPANDED_MATERIAL_FILTERED_WMO_COMPLETION_PROFILE,
            (
                MapSelection::All,
                run::MissingWdtPolicy::DeclareRootAbsent,
                run::MissingModelPolicy::DeclareRootAbsentWmoDoodadM2,
                run::MissingWmoPolicy::DeclareVacatedAfterRootAbsentM2OrEmptyOrUnselectedOrMaterialFiltered,
                resources::Profile::ModelOutputExpandedV1,
            ) => MODEL_OUTPUT_EXPANDED_MATERIAL_FILTERED_WMO_COMPLETION_PROFILE,
            (
                MapSelection::All,
                run::MissingWdtPolicy::DeclareRootAbsent,
                run::MissingModelPolicy::DeclareRootAbsentWmoDoodadM2,
                run::MissingWmoPolicy::DeclareVacatedAfterRootAbsentM2OrEmptyOrUnselectedOrMaterialFiltered,
                resources::Profile::ModelBytesExpandedV1,
            ) => MODEL_BYTES_EXPANDED_MATERIAL_FILTERED_WMO_COMPLETION_PROFILE,
            (
                MapSelection::All,
                run::MissingWdtPolicy::DeclareRootAbsent,
                run::MissingModelPolicy::DeclareRootAbsentWmoDoodadM2,
                run::MissingWmoPolicy::DeclareVacatedAfterRootAbsentM2OrEmptyOrUnselectedOrMaterialFiltered,
                resources::Profile::SourceExpandedV2,
            ) => SOURCE_EXPANDED_V2_MATERIAL_FILTERED_WMO_COMPLETION_PROFILE,
            (
                MapSelection::All,
                run::MissingWdtPolicy::DeclareRootAbsent,
                run::MissingModelPolicy::DeclareRootAbsentWmoDoodadM2,
                run::MissingWmoPolicy::DeclareVacatedAfterRootAbsentM2OrEmptyOrUnselectedOrMaterialFiltered,
                resources::Profile::PlacementExpandedV1,
            ) => PLACEMENT_EXPANDED_MATERIAL_FILTERED_WMO_COMPLETION_PROFILE,
            (
                MapSelection::All,
                run::MissingWdtPolicy::DeclareRootAbsent,
                run::MissingModelPolicy::DeclareRootAbsentWmoDoodadM2,
                run::MissingWmoPolicy::DeclareVacatedAfterRootAbsentM2OrEmptyOrUnselectedOrMaterialFiltered,
                resources::Profile::AssemblyExpandedV1,
            ) => ASSEMBLY_EXPANDED_MATERIAL_FILTERED_WMO_COMPLETION_PROFILE,
            (
                MapSelection::All,
                run::MissingWdtPolicy::DeclareRootAbsent,
                run::MissingModelPolicy::DeclareRootAbsentWmoDoodadM2,
                run::MissingWmoPolicy::DeclareVacatedAfterRootAbsentM2OrEmptyOrUnselectedOrMaterialFiltered,
                resources::Profile::GameObjectMissingRootExpandedV1,
            ) if allow_gameobject_missing_root && !allow_gameobject_encrypted_unavailable => {
                GAMEOBJECT_MISSING_ROOT_COMPLETION_PROFILE
            }
            (
                MapSelection::All,
                run::MissingWdtPolicy::DeclareRootAbsent,
                run::MissingModelPolicy::DeclareRootAbsentWmoDoodadM2,
                run::MissingWmoPolicy::DeclareVacatedAfterRootAbsentM2OrEmptyOrUnselectedOrMaterialFiltered,
                resources::Profile::GameObjectRootAndEncryptedUnavailableExpandedV1,
            ) if allow_gameobject_missing_root && allow_gameobject_encrypted_unavailable => {
                GAMEOBJECT_ROOT_ENCRYPTED_UNAVAILABLE_COMPLETION_PROFILE
            }
            (
                MapSelection::All,
                run::MissingWdtPolicy::DeclareRootAbsent,
                run::MissingModelPolicy::DeclareRootAbsentWmoDoodadM2,
                run::MissingWmoPolicy::DeclareVacatedAfterRootAbsentM2OrEmptyOrUnselectedOrMaterialFiltered,
                resources::Profile::GameObjectExpandedV1,
            ) => GAMEOBJECT_EXPANDED_MATERIAL_FILTERED_WMO_COMPLETION_PROFILE,
            _ => return Err(SCHEMA.into()),
            }
        };
        let source_missing_m2 = completed.world().missing_m2();
        validate_missing_m2(source_missing_m2, missing_m2_policy)?;
        validate_missing_model_artifacts(source_missing_m2, used_models, artifact_inventory)?;
        let mut missing_m2 = Vec::new();
        missing_m2
            .try_reserve_exact(source_missing_m2.len())
            .map_err(|_| Error::from("product_allocation"))?;
        missing_m2.extend_from_slice(source_missing_m2);
        let source_missing_wmo = completed.world().missing_wmo();
        let source_empty_wmo = completed.world().empty_wmo();
        let source_unselected_wmo = completed.world().unselected_wmo();
        let source_material_filtered_wmo = completed.world().material_filtered_wmo();
        validate_missing_wmo(source_missing_wmo, missing_wmo_policy, source_missing_m2)?;
        validate_empty_wmo(
            source_empty_wmo,
            missing_wmo_policy,
            source_missing_wmo,
            source_missing_m2,
        )?;
        validate_unselected_wmo(
            source_unselected_wmo,
            missing_wmo_policy,
            source_missing_m2,
            source_missing_wmo,
            source_empty_wmo,
        )?;
        let mut missing_wmo = Vec::new();
        missing_wmo
            .try_reserve_exact(source_missing_wmo.len())
            .map_err(|_| Error::from("product_allocation"))?;
        missing_wmo.extend_from_slice(source_missing_wmo);
        let mut empty_wmo = Vec::new();
        empty_wmo
            .try_reserve_exact(source_empty_wmo.len())
            .map_err(|_| Error::from("product_allocation"))?;
        empty_wmo.extend_from_slice(source_empty_wmo);
        let mut unselected_wmo = Vec::new();
        if matches!(
            missing_wmo_policy,
            run::MissingWmoPolicy::DeclareVacatedAfterRootAbsentM2OrEmptyOrUnselected
                | run::MissingWmoPolicy::DeclareVacatedAfterRootAbsentM2OrEmptyOrUnselectedOrMaterialFiltered
        ) {
            unselected_wmo
                .try_reserve_exact(source_unselected_wmo.len())
                .map_err(|_| Error::from("product_allocation"))?;
            unselected_wmo.extend_from_slice(source_unselected_wmo);
        }
        validate_material_filtered_wmo(
            source_material_filtered_wmo,
            missing_wmo_policy,
            source_missing_m2,
            source_missing_wmo,
            source_empty_wmo,
            &unselected_wmo,
        )?;
        validate_wmo_omission_artifacts(source_missing_wmo, used_models, artifact_inventory)?;
        validate_wmo_omission_artifacts(source_empty_wmo, used_models, artifact_inventory)?;
        validate_wmo_omission_artifacts(source_unselected_wmo, used_models, artifact_inventory)?;
        validate_wmo_omission_artifacts(
            source_material_filtered_wmo,
            used_models,
            artifact_inventory,
        )?;
        let mut material_filtered_wmo = Vec::new();
        if missing_wmo_policy
            == run::MissingWmoPolicy::DeclareVacatedAfterRootAbsentM2OrEmptyOrUnselectedOrMaterialFiltered
        {
            material_filtered_wmo
                .try_reserve_exact(source_material_filtered_wmo.len())
                .map_err(|_| Error::from("product_allocation"))?;
            material_filtered_wmo.extend_from_slice(source_material_filtered_wmo);
        }
        if missing_wmo_policy
            != run::MissingWmoPolicy::DeclareVacatedAfterRootAbsentM2OrEmptyOrUnselectedOrMaterialFiltered
            && !source_material_filtered_wmo.is_empty()
        {
            return Err(SCHEMA.into());
        }
        let facts = completed.coverage();
        let gameobject_profile = matches!(
            resource_profile,
            resources::Profile::GameObjectExpandedV1
                | resources::Profile::GameObjectMissingRootExpandedV1
                | resources::Profile::GameObjectRootAndEncryptedUnavailableExpandedV1
        );
        let (model_scope, gameobject_facts) = if gameobject_profile {
            let facts = completed.gameobject_facts().ok_or(SCHEMA)?;
            let coverage_facts = completed.coverage();
            let expected_gameobject_scope = if allow_gameobject_encrypted_unavailable {
                geometry::GameObjectScope::AvailableFdidDirectRootAbsentOrEncryptedUnavailableV1
            } else if allow_gameobject_missing_root {
                geometry::GameObjectScope::AvailableFdidDirectMissingRootV1
            } else {
                geometry::GameObjectScope::AvailableFdidDirectV1
            };
            let expected_model_scope = if allow_gameobject_encrypted_unavailable {
                GAMEOBJECT_ROOT_ENCRYPTED_UNAVAILABLE_MODEL_SCOPE_PROFILE
            } else if allow_gameobject_missing_root {
                GAMEOBJECT_MISSING_ROOT_MODEL_SCOPE_PROFILE
            } else {
                GAMEOBJECT_MODEL_SCOPE_PROFILE
            };
            if coverage_facts.gameobject_scope != expected_gameobject_scope
                || coverage_facts.gameobject_file_data_id
                    != forever_casc::collision::gameobjects::TABLE_FILE_DATA_ID
                || coverage_facts.gameobject_block_reason != geometry::GameObjectBlockReason::None
                || !coverage_facts.gameobject_request_made
                || coverage_facts.liquid_object_scope != geometry::LiquidObjectScope::OutOfScope
                || coverage_facts.liquid_object_request_made
                || coverage_facts.model_scope != expected_model_scope
                || coverage_facts.gameobject_startup_reads == 0
                || (allow_gameobject_missing_root
                    && (facts.missing_root_fdids != completed.gameobjects().missing_root()
                        || facts.missing_root_rows as usize
                            != completed.gameobjects().missing_root_rows()))
                || (!allow_gameobject_missing_root
                    && (facts.missing_root_rows != 0 || !facts.missing_root_fdids.is_empty()))
                || (allow_gameobject_encrypted_unavailable
                    && (facts.encrypted_unavailable_fdids
                        != completed.gameobjects().encrypted_unavailable()
                        || facts.encrypted_unavailable_rows as usize
                            != completed.gameobjects().encrypted_unavailable_rows()))
                || (!allow_gameobject_encrypted_unavailable
                    && (facts.encrypted_unavailable_rows != 0
                        || !facts.encrypted_unavailable_fdids.is_empty()))
                || facts.direct_outcomes != coverage_facts.gameobject_outcomes
                || facts.direct_rows != coverage_facts.gameobject_direct_rows
                || facts.copy_records != coverage_facts.gameobject_copy_records
                || facts.sections.len() as u64 != coverage_facts.gameobject_sections
            {
                return Err(SCHEMA.into());
            }
            (
                if allow_gameobject_encrypted_unavailable {
                    ModelScope::PlacementAndGameObjectRootAndEncryptedUnavailable
                } else if allow_gameobject_missing_root {
                    ModelScope::PlacementAndGameObjectMissingRoot
                } else {
                    ModelScope::PlacementAndGameObject
                },
                Some(gameobject_facts_from_geometry(facts)?),
            )
        } else {
            if facts.gameobject_scope != geometry::GameObjectScope::Excluded
                || facts.gameobject_file_data_id
                    != forever_casc::collision::gameobjects::TABLE_FILE_DATA_ID
                || facts.gameobject_block_reason
                    != geometry::GameObjectBlockReason::ExcludedByProfile
                || facts.gameobject_request_made
                || completed.gameobject_facts().is_some()
                || facts.liquid_object_scope != geometry::LiquidObjectScope::OutOfScope
                || facts.liquid_object_request_made
            {
                return Err(SCHEMA.into());
            }
            let (scope, startup_reads) =
                derive_model_scope(completed, used_models, artifact_inventory)?;
            if scope != ModelScope::PlacementOnly || startup_reads != facts.gameobject_startup_reads
            {
                return Err(SCHEMA.into());
            }
            (scope, None)
        };
        if completed.profile() != expected_profile || completed.selection() != selection {
            return Err(SCHEMA.into());
        }
        let map = available_map_fact(completed.world().map())?;
        let source_liquids = completed.world().liquids();
        let liquids = [
            liquid_fact(&source_liquids[0])?,
            liquid_fact(&source_liquids[1])?,
        ];
        if liquids[0].file_data_id != world::MATERIAL_FILE_DATA_ID
            || liquids[1].file_data_id != world::TYPE_FILE_DATA_ID
        {
            return Err(SCHEMA.into());
        }
        let maps = completed_domain(completed, policy)?;
        validate_omitted_artifacts(&maps, artifact_inventory)?;
        if gameobject_profile {
            let gameobjects = completed.gameobject_facts().ok_or(SCHEMA)?;
            let dtree = artifact_inventory
                .get(GAMEOBJECT_MODELS_DTREE_PATH)
                .ok_or(SCHEMA)?;
            if dtree.bytes != gameobjects.dtree_bytes || dtree.sha != gameobjects.dtree_sha256 {
                return Err(SCHEMA.into());
            }
            if allow_gameobject_missing_root {
                validate_missing_model_artifacts(
                    &gameobjects.missing_root_fdids,
                    used_models,
                    artifact_inventory,
                )?;
            }
        } else if artifact_inventory.contains_key(GAMEOBJECT_MODELS_DTREE_PATH)
            || completed.gameobject_facts().is_some()
        {
            return Err(SCHEMA.into());
        }
        if artifact_inventory.len() > COVERAGE_MAX_ARTIFACTS {
            return Err(LIMIT.into());
        }
        let mut artifacts = Vec::new();
        artifacts
            .try_reserve_exact(artifact_inventory.len())
            .map_err(|_| Error::from("product_allocation"))?;
        // BTreeMap traversal is already the canonical bytewise path order.
        for fact in artifact_inventory.values() {
            if artifacts.len() == COVERAGE_MAX_ARTIFACTS {
                return Err(LIMIT.into());
            }
            artifacts.push(fact.clone());
        }
        Ok(Self {
            context,
            selection,
            missing_wdt_policy,
            missing_m2_policy,
            missing_wmo_policy,
            resource_profile,
            missing_m2,
            missing_wmo,
            empty_wmo,
            unselected_wmo,
            material_filtered_wmo,
            gameobject_missing_root: if policy.allow_gameobject_missing_root {
                let missing_root = &gameobject_facts.as_ref().ok_or(SCHEMA)?.missing_root_fdids;
                if missing_root.len() > GAMEOBJECT_MISSING_ROOT_MAX
                    || missing_root.windows(2).any(|pair| pair[0] >= pair[1])
                {
                    return Err(SCHEMA.into());
                }
                missing_root.clone()
            } else {
                Vec::new()
            },
            gameobject_encrypted_unavailable: if policy.allow_gameobject_encrypted_unavailable {
                let encrypted = &gameobject_facts
                    .as_ref()
                    .ok_or(SCHEMA)?
                    .encrypted_unavailable_fdids;
                if encrypted.len() > GAMEOBJECT_ENCRYPTED_UNAVAILABLE_MAX
                    || encrypted.windows(2).any(|pair| pair[0] >= pair[1])
                {
                    return Err(SCHEMA.into());
                }
                encrypted.clone()
            } else {
                Vec::new()
            },
            gameobject_scope: facts.gameobject_scope,
            gameobject_file_data_id,
            gameobject_block_reason: facts.gameobject_block_reason,
            gameobject_request_made: facts.gameobject_request_made,
            gameobject_startup_reads: facts.gameobject_startup_reads,
            gameobjects: gameobject_facts,
            model_scope,
            liquid_object_scope: facts.liquid_object_scope,
            liquid_object_request_made: facts.liquid_object_request_made,
            map,
            liquids,
            maps,
            artifacts,
        })
    }

    pub(crate) fn encode(&self) -> Result<Vec<u8>, Error> {
        let version = coverage_version(
            GeometryPolicy::new_with_missing_wmo(
                self.selection,
                self.missing_wdt_policy,
                self.missing_m2_policy,
                self.missing_wmo_policy,
                self.resource_profile,
            )
            .with_gameobject_absence_if(matches!(
                self.resource_profile,
                resources::Profile::GameObjectMissingRootExpandedV1
                    | resources::Profile::GameObjectRootAndEncryptedUnavailableExpandedV1
            )),
        )?;
        if (!matches!(
            version,
            ROOT_ABSENT_COVERAGE_VERSION
                | MODEL_ROOT_ABSENT_COVERAGE_VERSION
                | REQUEST_CAPACITY_COVERAGE_VERSION
                | VACATED_WMO_COVERAGE_VERSION
                | EMPTY_WMO_COVERAGE_VERSION
                | UNSELECTED_WMO_COVERAGE_VERSION
                | MATERIAL_FILTERED_WMO_COVERAGE_VERSION
                | DEPENDENCY_HEAVY_MATERIAL_FILTERED_WMO_COVERAGE_VERSION
                | SOURCE_HEAVY_MATERIAL_FILTERED_WMO_COVERAGE_VERSION
                | READ_HEAVY_MATERIAL_FILTERED_WMO_COVERAGE_VERSION
                | MODEL_HEAVY_MATERIAL_FILTERED_WMO_COVERAGE_VERSION
                | SOURCE_EXPANDED_MATERIAL_FILTERED_WMO_COVERAGE_VERSION
                | MODEL_BYTES_EXPANDED_MATERIAL_FILTERED_WMO_COVERAGE_VERSION
                | SOURCE_EXPANDED_V2_MATERIAL_FILTERED_WMO_COVERAGE_VERSION
                | PLACEMENT_EXPANDED_MATERIAL_FILTERED_WMO_COVERAGE_VERSION
                | ASSEMBLY_EXPANDED_MATERIAL_FILTERED_WMO_COVERAGE_VERSION
                | MODEL_OUTPUT_EXPANDED_MATERIAL_FILTERED_WMO_COVERAGE_VERSION
                | GAMEOBJECT_EXPANDED_MATERIAL_FILTERED_WMO_COVERAGE_VERSION
                | GAMEOBJECT_MISSING_ROOT_COVERAGE_VERSION
                | GAMEOBJECT_ROOT_ENCRYPTED_UNAVAILABLE_COVERAGE_VERSION
        ) && self.maps.iter().any(|map| map.root_absent))
            || (!matches!(
                version,
                MODEL_ROOT_ABSENT_COVERAGE_VERSION
                    | REQUEST_CAPACITY_COVERAGE_VERSION
                    | VACATED_WMO_COVERAGE_VERSION
                    | EMPTY_WMO_COVERAGE_VERSION
                    | UNSELECTED_WMO_COVERAGE_VERSION
                    | MATERIAL_FILTERED_WMO_COVERAGE_VERSION
                    | DEPENDENCY_HEAVY_MATERIAL_FILTERED_WMO_COVERAGE_VERSION
                    | SOURCE_HEAVY_MATERIAL_FILTERED_WMO_COVERAGE_VERSION
                    | READ_HEAVY_MATERIAL_FILTERED_WMO_COVERAGE_VERSION
                    | MODEL_HEAVY_MATERIAL_FILTERED_WMO_COVERAGE_VERSION
                    | SOURCE_EXPANDED_MATERIAL_FILTERED_WMO_COVERAGE_VERSION
                    | MODEL_BYTES_EXPANDED_MATERIAL_FILTERED_WMO_COVERAGE_VERSION
                    | SOURCE_EXPANDED_V2_MATERIAL_FILTERED_WMO_COVERAGE_VERSION
                    | PLACEMENT_EXPANDED_MATERIAL_FILTERED_WMO_COVERAGE_VERSION
                    | ASSEMBLY_EXPANDED_MATERIAL_FILTERED_WMO_COVERAGE_VERSION
                    | MODEL_OUTPUT_EXPANDED_MATERIAL_FILTERED_WMO_COVERAGE_VERSION
                    | GAMEOBJECT_EXPANDED_MATERIAL_FILTERED_WMO_COVERAGE_VERSION
                    | GAMEOBJECT_MISSING_ROOT_COVERAGE_VERSION
                    | GAMEOBJECT_ROOT_ENCRYPTED_UNAVAILABLE_COVERAGE_VERSION
            ) && !self.missing_m2.is_empty())
        {
            return Err(SCHEMA.into());
        }
        if matches!(
            version,
            VACATED_WMO_COVERAGE_VERSION
                | EMPTY_WMO_COVERAGE_VERSION
                | UNSELECTED_WMO_COVERAGE_VERSION
                | MATERIAL_FILTERED_WMO_COVERAGE_VERSION
                | DEPENDENCY_HEAVY_MATERIAL_FILTERED_WMO_COVERAGE_VERSION
                | SOURCE_HEAVY_MATERIAL_FILTERED_WMO_COVERAGE_VERSION
                | READ_HEAVY_MATERIAL_FILTERED_WMO_COVERAGE_VERSION
                | MODEL_HEAVY_MATERIAL_FILTERED_WMO_COVERAGE_VERSION
                | SOURCE_EXPANDED_MATERIAL_FILTERED_WMO_COVERAGE_VERSION
                | MODEL_BYTES_EXPANDED_MATERIAL_FILTERED_WMO_COVERAGE_VERSION
                | SOURCE_EXPANDED_V2_MATERIAL_FILTERED_WMO_COVERAGE_VERSION
                | PLACEMENT_EXPANDED_MATERIAL_FILTERED_WMO_COVERAGE_VERSION
                | ASSEMBLY_EXPANDED_MATERIAL_FILTERED_WMO_COVERAGE_VERSION
                | MODEL_OUTPUT_EXPANDED_MATERIAL_FILTERED_WMO_COVERAGE_VERSION
                | GAMEOBJECT_EXPANDED_MATERIAL_FILTERED_WMO_COVERAGE_VERSION
                | GAMEOBJECT_MISSING_ROOT_COVERAGE_VERSION
                | GAMEOBJECT_ROOT_ENCRYPTED_UNAVAILABLE_COVERAGE_VERSION
        ) {
            validate_missing_m2(&self.missing_m2, self.missing_m2_policy)?;
            validate_missing_wmo(&self.missing_wmo, self.missing_wmo_policy, &self.missing_m2)?;
            validate_empty_wmo(
                &self.empty_wmo,
                self.missing_wmo_policy,
                &self.missing_wmo,
                &self.missing_m2,
            )?;
            if matches!(
                version,
                UNSELECTED_WMO_COVERAGE_VERSION
                    | MATERIAL_FILTERED_WMO_COVERAGE_VERSION
                    | DEPENDENCY_HEAVY_MATERIAL_FILTERED_WMO_COVERAGE_VERSION
                    | SOURCE_HEAVY_MATERIAL_FILTERED_WMO_COVERAGE_VERSION
                    | READ_HEAVY_MATERIAL_FILTERED_WMO_COVERAGE_VERSION
                    | MODEL_HEAVY_MATERIAL_FILTERED_WMO_COVERAGE_VERSION
                    | SOURCE_EXPANDED_MATERIAL_FILTERED_WMO_COVERAGE_VERSION
                    | MODEL_BYTES_EXPANDED_MATERIAL_FILTERED_WMO_COVERAGE_VERSION
                    | SOURCE_EXPANDED_V2_MATERIAL_FILTERED_WMO_COVERAGE_VERSION
                    | PLACEMENT_EXPANDED_MATERIAL_FILTERED_WMO_COVERAGE_VERSION
                    | ASSEMBLY_EXPANDED_MATERIAL_FILTERED_WMO_COVERAGE_VERSION
                    | MODEL_OUTPUT_EXPANDED_MATERIAL_FILTERED_WMO_COVERAGE_VERSION
                    | GAMEOBJECT_EXPANDED_MATERIAL_FILTERED_WMO_COVERAGE_VERSION
                    | GAMEOBJECT_MISSING_ROOT_COVERAGE_VERSION
                    | GAMEOBJECT_ROOT_ENCRYPTED_UNAVAILABLE_COVERAGE_VERSION
            ) {
                validate_unselected_wmo(
                    &self.unselected_wmo,
                    self.missing_wmo_policy,
                    &self.missing_m2,
                    &self.missing_wmo,
                    &self.empty_wmo,
                )?;
            } else if !self.unselected_wmo.is_empty() {
                return Err(SCHEMA.into());
            }
            if matches!(
                version,
                MATERIAL_FILTERED_WMO_COVERAGE_VERSION
                    | DEPENDENCY_HEAVY_MATERIAL_FILTERED_WMO_COVERAGE_VERSION
                    | SOURCE_HEAVY_MATERIAL_FILTERED_WMO_COVERAGE_VERSION
                    | READ_HEAVY_MATERIAL_FILTERED_WMO_COVERAGE_VERSION
                    | MODEL_HEAVY_MATERIAL_FILTERED_WMO_COVERAGE_VERSION
                    | SOURCE_EXPANDED_MATERIAL_FILTERED_WMO_COVERAGE_VERSION
                    | MODEL_BYTES_EXPANDED_MATERIAL_FILTERED_WMO_COVERAGE_VERSION
                    | SOURCE_EXPANDED_V2_MATERIAL_FILTERED_WMO_COVERAGE_VERSION
                    | PLACEMENT_EXPANDED_MATERIAL_FILTERED_WMO_COVERAGE_VERSION
                    | ASSEMBLY_EXPANDED_MATERIAL_FILTERED_WMO_COVERAGE_VERSION
                    | MODEL_OUTPUT_EXPANDED_MATERIAL_FILTERED_WMO_COVERAGE_VERSION
                    | GAMEOBJECT_EXPANDED_MATERIAL_FILTERED_WMO_COVERAGE_VERSION
                    | GAMEOBJECT_MISSING_ROOT_COVERAGE_VERSION
                    | GAMEOBJECT_ROOT_ENCRYPTED_UNAVAILABLE_COVERAGE_VERSION
            ) {
                validate_material_filtered_wmo(
                    &self.material_filtered_wmo,
                    self.missing_wmo_policy,
                    &self.missing_m2,
                    &self.missing_wmo,
                    &self.empty_wmo,
                    &self.unselected_wmo,
                )?;
            } else if !self.material_filtered_wmo.is_empty() {
                return Err(SCHEMA.into());
            }
        } else if self.missing_wmo_policy != run::MissingWmoPolicy::Reject
            || !self.missing_wmo.is_empty()
            || !self.empty_wmo.is_empty()
            || !self.unselected_wmo.is_empty()
            || !self.material_filtered_wmo.is_empty()
        {
            return Err(SCHEMA.into());
        }
        if matches!(
            version,
            GAMEOBJECT_MISSING_ROOT_COVERAGE_VERSION
                | GAMEOBJECT_ROOT_ENCRYPTED_UNAVAILABLE_COVERAGE_VERSION
        ) {
            let facts = self.gameobjects.as_ref().ok_or(SCHEMA)?;
            let allow_encrypted_unavailable =
                version == GAMEOBJECT_ROOT_ENCRYPTED_UNAVAILABLE_COVERAGE_VERSION;
            if validate_gameobject_facts(facts, true, allow_encrypted_unavailable).is_err()
                || self.gameobject_missing_root != facts.missing_root_fdids
                || self.gameobject_missing_root.len() > GAMEOBJECT_MISSING_ROOT_MAX
                || self.gameobject_missing_root.contains(&0)
                || self
                    .gameobject_missing_root
                    .windows(2)
                    .any(|pair| pair[0] >= pair[1])
                || self.gameobject_encrypted_unavailable != facts.encrypted_unavailable_fdids
                || (!allow_encrypted_unavailable
                    && (!self.gameobject_encrypted_unavailable.is_empty()
                        || facts.encrypted_unavailable_rows != 0))
                || self.gameobject_encrypted_unavailable.len()
                    > GAMEOBJECT_ENCRYPTED_UNAVAILABLE_MAX
                || self.gameobject_encrypted_unavailable.contains(&0)
                || self
                    .gameobject_encrypted_unavailable
                    .windows(2)
                    .any(|pair| pair[0] >= pair[1])
                || self.gameobject_missing_root.iter().any(|id| {
                    self.gameobject_encrypted_unavailable
                        .binary_search(id)
                        .is_ok()
                        || self.missing_m2.binary_search(id).is_ok()
                        || self.missing_wmo.binary_search(id).is_ok()
                        || self.empty_wmo.binary_search(id).is_ok()
                        || self.unselected_wmo.binary_search(id).is_ok()
                        || self.material_filtered_wmo.binary_search(id).is_ok()
                })
                || self.gameobject_encrypted_unavailable.iter().any(|id| {
                    self.missing_m2.binary_search(id).is_ok()
                        || self.missing_wmo.binary_search(id).is_ok()
                        || self.empty_wmo.binary_search(id).is_ok()
                        || self.unselected_wmo.binary_search(id).is_ok()
                        || self.material_filtered_wmo.binary_search(id).is_ok()
                })
            {
                return Err(SCHEMA.into());
            }
        } else if !self.gameobject_missing_root.is_empty()
            || !self.gameobject_encrypted_unavailable.is_empty()
        {
            return Err(SCHEMA.into());
        }
        let mut writer = Writer::new(COVERAGE_MAX_BYTES);
        writer.put(COVERAGE_MAGIC)?;
        writer.u32(version)?;
        writer.text(&self.context.json())?;
        match (
            self.selection,
            self.missing_wdt_policy,
            self.missing_m2_policy,
            self.missing_wmo_policy,
            self.resource_profile,
        ) {
            (
                MapSelection::MapAndAncestors(id),
                run::MissingWdtPolicy::Reject,
                run::MissingModelPolicy::Reject,
                run::MissingWmoPolicy::Reject,
                resources::Profile::BoundedV1,
            ) => {
                writer.u32(id)?;
                writer.text(crate::plan::SELECTED_GEOMETRY_COVERAGE_PROFILE)?;
                writer.text(PROFILE)?;
                writer.text("available-map-active-ancestors-v1")?;
            }
            (
                MapSelection::All,
                run::MissingWdtPolicy::Reject,
                run::MissingModelPolicy::Reject,
                run::MissingWmoPolicy::Reject,
                resources::Profile::BoundedV1,
            ) => {
                writer.text(crate::plan::AVAILABLE_GEOMETRY_COVERAGE_PROFILE)?;
                writer.text(AVAILABLE_COVERAGE_PROFILE)?;
                writer.text("available-map-all-active-v1")?;
            }
            (
                MapSelection::All,
                run::MissingWdtPolicy::DeclareRootAbsent,
                run::MissingModelPolicy::Reject,
                run::MissingWmoPolicy::Reject,
                resources::Profile::BoundedV1,
            ) => {
                writer.text(crate::plan::ROOT_ABSENT_GEOMETRY_COVERAGE_PROFILE)?;
                writer.text(ROOT_ABSENT_COVERAGE_PROFILE)?;
                writer.text(ROOT_ABSENT_DOMAIN_PROFILE)?;
            }
            (
                MapSelection::All,
                run::MissingWdtPolicy::DeclareRootAbsent,
                run::MissingModelPolicy::DeclareRootAbsentWmoDoodadM2,
                run::MissingWmoPolicy::Reject,
                resources::Profile::BoundedV1,
            ) => {
                writer.text(crate::plan::MODEL_ROOT_ABSENT_GEOMETRY_COVERAGE_PROFILE)?;
                writer.text(MODEL_ROOT_ABSENT_COVERAGE_PROFILE)?;
                writer.text(MODEL_ROOT_ABSENT_DOMAIN_PROFILE)?;
            }
            (
                MapSelection::All,
                run::MissingWdtPolicy::DeclareRootAbsent,
                run::MissingModelPolicy::DeclareRootAbsentWmoDoodadM2,
                run::MissingWmoPolicy::Reject,
                resources::Profile::RequestHeavyV1,
            ) => {
                writer.text(
                    crate::plan::MODEL_ROOT_ABSENT_REQUEST_CAPACITY_GEOMETRY_COVERAGE_PROFILE,
                )?;
                writer.text(REQUEST_CAPACITY_COVERAGE_PROFILE)?;
                writer.text(REQUEST_CAPACITY_DOMAIN_PROFILE)?;
            }
            (
                MapSelection::All,
                run::MissingWdtPolicy::DeclareRootAbsent,
                run::MissingModelPolicy::DeclareRootAbsentWmoDoodadM2,
                run::MissingWmoPolicy::DeclareVacatedAfterRootAbsentM2,
                resources::Profile::RequestHeavyV1,
            ) => {
                writer.text(crate::plan::VACATED_WMO_COVERAGE_PROFILE)?;
                writer.text(VACATED_WMO_CODEC_PROFILE)?;
                writer.text(VACATED_WMO_DOMAIN_PROFILE)?;
            }
            (
                MapSelection::All,
                run::MissingWdtPolicy::DeclareRootAbsent,
                run::MissingModelPolicy::DeclareRootAbsentWmoDoodadM2,
                run::MissingWmoPolicy::DeclareVacatedAfterRootAbsentM2OrEmpty,
                resources::Profile::RequestHeavyV1,
            ) => {
                writer.text(crate::plan::EMPTY_WMO_COVERAGE_PROFILE)?;
                writer.text(EMPTY_WMO_CODEC_PROFILE)?;
                writer.text(EMPTY_WMO_DOMAIN_PROFILE)?;
            }
            (
                MapSelection::All,
                run::MissingWdtPolicy::DeclareRootAbsent,
                run::MissingModelPolicy::DeclareRootAbsentWmoDoodadM2,
                run::MissingWmoPolicy::DeclareVacatedAfterRootAbsentM2OrEmptyOrUnselected,
                resources::Profile::RequestHeavyV1,
            ) => {
                writer.text(crate::plan::UNSELECTED_WMO_COVERAGE_PROFILE)?;
                writer.text(UNSELECTED_WMO_CODEC_PROFILE)?;
                writer.text(UNSELECTED_WMO_DOMAIN_PROFILE)?;
            }
            (
                MapSelection::All,
                run::MissingWdtPolicy::DeclareRootAbsent,
                run::MissingModelPolicy::DeclareRootAbsentWmoDoodadM2,
                run::MissingWmoPolicy::DeclareVacatedAfterRootAbsentM2OrEmptyOrUnselectedOrMaterialFiltered,
                resources::Profile::RequestHeavyV1,
            ) => {
                writer.text(crate::plan::MATERIAL_FILTERED_WMO_COVERAGE_PROFILE)?;
                writer.text(MATERIAL_FILTERED_WMO_CODEC_PROFILE)?;
                writer.text(MATERIAL_FILTERED_WMO_DOMAIN_PROFILE)?;
            }
            (
                MapSelection::All,
                run::MissingWdtPolicy::DeclareRootAbsent,
                run::MissingModelPolicy::DeclareRootAbsentWmoDoodadM2,
                run::MissingWmoPolicy::DeclareVacatedAfterRootAbsentM2OrEmptyOrUnselectedOrMaterialFiltered,
                resources::Profile::DependencyHeavyV1,
            ) => {
                writer.text(crate::plan::DEPENDENCY_HEAVY_MATERIAL_FILTERED_WMO_COVERAGE_PROFILE)?;
                writer.text(DEPENDENCY_HEAVY_MATERIAL_FILTERED_WMO_CODEC_PROFILE)?;
                writer.text(DEPENDENCY_HEAVY_MATERIAL_FILTERED_WMO_DOMAIN_PROFILE)?;
            }
            (
                MapSelection::All,
                run::MissingWdtPolicy::DeclareRootAbsent,
                run::MissingModelPolicy::DeclareRootAbsentWmoDoodadM2,
                run::MissingWmoPolicy::DeclareVacatedAfterRootAbsentM2OrEmptyOrUnselectedOrMaterialFiltered,
                resources::Profile::SourceHeavyV1,
            ) => {
                writer.text(crate::plan::SOURCE_HEAVY_MATERIAL_FILTERED_WMO_COVERAGE_PROFILE)?;
                writer.text(SOURCE_HEAVY_MATERIAL_FILTERED_WMO_CODEC_PROFILE)?;
                writer.text(SOURCE_HEAVY_MATERIAL_FILTERED_WMO_DOMAIN_PROFILE)?;
            }
            (
                MapSelection::All,
                run::MissingWdtPolicy::DeclareRootAbsent,
                run::MissingModelPolicy::DeclareRootAbsentWmoDoodadM2,
                run::MissingWmoPolicy::DeclareVacatedAfterRootAbsentM2OrEmptyOrUnselectedOrMaterialFiltered,
                resources::Profile::ReadHeavyV1,
            ) => {
                writer.text(crate::plan::READ_HEAVY_MATERIAL_FILTERED_WMO_COVERAGE_PROFILE)?;
                writer.text(READ_HEAVY_MATERIAL_FILTERED_WMO_CODEC_PROFILE)?;
                writer.text(READ_HEAVY_MATERIAL_FILTERED_WMO_DOMAIN_PROFILE)?;
            }
            (
                MapSelection::All,
                run::MissingWdtPolicy::DeclareRootAbsent,
                run::MissingModelPolicy::DeclareRootAbsentWmoDoodadM2,
                run::MissingWmoPolicy::DeclareVacatedAfterRootAbsentM2OrEmptyOrUnselectedOrMaterialFiltered,
                resources::Profile::ModelHeavyV1,
            ) => {
                writer.text(crate::plan::MODEL_HEAVY_MATERIAL_FILTERED_WMO_COVERAGE_PROFILE)?;
                writer.text(MODEL_HEAVY_MATERIAL_FILTERED_WMO_CODEC_PROFILE)?;
                writer.text(MODEL_HEAVY_MATERIAL_FILTERED_WMO_DOMAIN_PROFILE)?;
            }
            (
                MapSelection::All,
                run::MissingWdtPolicy::DeclareRootAbsent,
                run::MissingModelPolicy::DeclareRootAbsentWmoDoodadM2,
                run::MissingWmoPolicy::DeclareVacatedAfterRootAbsentM2OrEmptyOrUnselectedOrMaterialFiltered,
                resources::Profile::SourceExpandedV1,
            ) => {
                writer.text(crate::plan::SOURCE_EXPANDED_MATERIAL_FILTERED_WMO_COVERAGE_PROFILE)?;
                writer.text(SOURCE_EXPANDED_MATERIAL_FILTERED_WMO_CODEC_PROFILE)?;
                writer.text(SOURCE_EXPANDED_MATERIAL_FILTERED_WMO_DOMAIN_PROFILE)?;
            }
            (
                MapSelection::All,
                run::MissingWdtPolicy::DeclareRootAbsent,
                run::MissingModelPolicy::DeclareRootAbsentWmoDoodadM2,
                run::MissingWmoPolicy::DeclareVacatedAfterRootAbsentM2OrEmptyOrUnselectedOrMaterialFiltered,
                resources::Profile::ModelOutputExpandedV1,
            ) => {
                writer.text(crate::plan::MODEL_OUTPUT_EXPANDED_MATERIAL_FILTERED_WMO_COVERAGE_PROFILE)?;
                writer.text(MODEL_OUTPUT_EXPANDED_MATERIAL_FILTERED_WMO_CODEC_PROFILE)?;
                writer.text(MODEL_OUTPUT_EXPANDED_MATERIAL_FILTERED_WMO_DOMAIN_PROFILE)?;
            }
            (
                MapSelection::All,
                run::MissingWdtPolicy::DeclareRootAbsent,
                run::MissingModelPolicy::DeclareRootAbsentWmoDoodadM2,
                run::MissingWmoPolicy::DeclareVacatedAfterRootAbsentM2OrEmptyOrUnselectedOrMaterialFiltered,
                resources::Profile::ModelBytesExpandedV1,
            ) => {
                writer.text(crate::plan::MODEL_BYTES_EXPANDED_MATERIAL_FILTERED_WMO_COVERAGE_PROFILE)?;
                writer.text(MODEL_BYTES_EXPANDED_MATERIAL_FILTERED_WMO_CODEC_PROFILE)?;
                writer.text(MODEL_BYTES_EXPANDED_MATERIAL_FILTERED_WMO_DOMAIN_PROFILE)?;
            }
            (
                MapSelection::All,
                run::MissingWdtPolicy::DeclareRootAbsent,
                run::MissingModelPolicy::DeclareRootAbsentWmoDoodadM2,
                run::MissingWmoPolicy::DeclareVacatedAfterRootAbsentM2OrEmptyOrUnselectedOrMaterialFiltered,
                resources::Profile::SourceExpandedV2,
            ) => {
                writer.text(crate::plan::SOURCE_EXPANDED_V2_MATERIAL_FILTERED_WMO_COVERAGE_PROFILE)?;
                writer.text(SOURCE_EXPANDED_V2_MATERIAL_FILTERED_WMO_CODEC_PROFILE)?;
                writer.text(SOURCE_EXPANDED_V2_MATERIAL_FILTERED_WMO_DOMAIN_PROFILE)?;
            }
            (
                MapSelection::All,
                run::MissingWdtPolicy::DeclareRootAbsent,
                run::MissingModelPolicy::DeclareRootAbsentWmoDoodadM2,
                run::MissingWmoPolicy::DeclareVacatedAfterRootAbsentM2OrEmptyOrUnselectedOrMaterialFiltered,
                resources::Profile::PlacementExpandedV1,
            ) => {
                writer.text(crate::plan::PLACEMENT_EXPANDED_MATERIAL_FILTERED_WMO_COVERAGE_PROFILE)?;
                writer.text(PLACEMENT_EXPANDED_MATERIAL_FILTERED_WMO_CODEC_PROFILE)?;
                writer.text(PLACEMENT_EXPANDED_MATERIAL_FILTERED_WMO_DOMAIN_PROFILE)?;
            }
            (
                MapSelection::All,
                run::MissingWdtPolicy::DeclareRootAbsent,
                run::MissingModelPolicy::DeclareRootAbsentWmoDoodadM2,
                run::MissingWmoPolicy::DeclareVacatedAfterRootAbsentM2OrEmptyOrUnselectedOrMaterialFiltered,
                resources::Profile::AssemblyExpandedV1,
            ) => {
                writer.text(crate::plan::ASSEMBLY_EXPANDED_MATERIAL_FILTERED_WMO_COVERAGE_PROFILE)?;
                writer.text(ASSEMBLY_EXPANDED_MATERIAL_FILTERED_WMO_CODEC_PROFILE)?;
                writer.text(ASSEMBLY_EXPANDED_MATERIAL_FILTERED_WMO_DOMAIN_PROFILE)?;
            }
            (
                MapSelection::All,
                run::MissingWdtPolicy::DeclareRootAbsent,
                run::MissingModelPolicy::DeclareRootAbsentWmoDoodadM2,
                run::MissingWmoPolicy::DeclareVacatedAfterRootAbsentM2OrEmptyOrUnselectedOrMaterialFiltered,
                resources::Profile::GameObjectExpandedV1,
            ) => {
                writer.text(crate::plan::GAMEOBJECT_EXPANDED_MATERIAL_FILTERED_WMO_COVERAGE_PROFILE)?;
                writer.text(GAMEOBJECT_EXPANDED_MATERIAL_FILTERED_WMO_CODEC_PROFILE)?;
                writer.text(GAMEOBJECT_EXPANDED_MATERIAL_FILTERED_WMO_DOMAIN_PROFILE)?;
            }
            (
                MapSelection::All,
                run::MissingWdtPolicy::DeclareRootAbsent,
                run::MissingModelPolicy::DeclareRootAbsentWmoDoodadM2,
                run::MissingWmoPolicy::DeclareVacatedAfterRootAbsentM2OrEmptyOrUnselectedOrMaterialFiltered,
                resources::Profile::GameObjectMissingRootExpandedV1,
            ) => {
                writer.text(crate::plan::GAMEOBJECT_MISSING_ROOT_MATERIAL_FILTERED_WMO_COVERAGE_PROFILE)?;
                writer.text(GAMEOBJECT_MISSING_ROOT_CODEC_PROFILE)?;
                writer.text(GAMEOBJECT_MISSING_ROOT_DOMAIN_PROFILE)?;
            }
            (
                MapSelection::All,
                run::MissingWdtPolicy::DeclareRootAbsent,
                run::MissingModelPolicy::DeclareRootAbsentWmoDoodadM2,
                run::MissingWmoPolicy::DeclareVacatedAfterRootAbsentM2OrEmptyOrUnselectedOrMaterialFiltered,
                resources::Profile::GameObjectRootAndEncryptedUnavailableExpandedV1,
            ) => {
                if self.context.components == 25 {
                    writer.text(crate::plan::DB2_TERRAIN_LIQUID_VMAP_COVERAGE_PROFILE)?;
                    writer.text(DB2_TERRAIN_LIQUID_VMAP_CODEC_PROFILE)?;
                    writer.text(crate::plan::DB2_TERRAIN_LIQUID_VMAP_DOMAIN_PROFILE)?;
                } else {
                    writer.text(crate::plan::GAMEOBJECT_ROOT_ENCRYPTED_UNAVAILABLE_MATERIAL_FILTERED_WMO_COVERAGE_PROFILE)?;
                    writer.text(GAMEOBJECT_ROOT_ENCRYPTED_UNAVAILABLE_CODEC_PROFILE)?;
                    writer.text(GAMEOBJECT_ROOT_ENCRYPTED_UNAVAILABLE_DOMAIN_PROFILE)?;
                }
            }
            _ => return Err(SCHEMA.into()),
        }
        if matches!(
            version,
            ROOT_ABSENT_COVERAGE_VERSION
                | MODEL_ROOT_ABSENT_COVERAGE_VERSION
                | REQUEST_CAPACITY_COVERAGE_VERSION
                | VACATED_WMO_COVERAGE_VERSION
                | EMPTY_WMO_COVERAGE_VERSION
                | UNSELECTED_WMO_COVERAGE_VERSION
                | MATERIAL_FILTERED_WMO_COVERAGE_VERSION
                | DEPENDENCY_HEAVY_MATERIAL_FILTERED_WMO_COVERAGE_VERSION
                | SOURCE_HEAVY_MATERIAL_FILTERED_WMO_COVERAGE_VERSION
                | READ_HEAVY_MATERIAL_FILTERED_WMO_COVERAGE_VERSION
                | MODEL_HEAVY_MATERIAL_FILTERED_WMO_COVERAGE_VERSION
                | SOURCE_EXPANDED_MATERIAL_FILTERED_WMO_COVERAGE_VERSION
                | MODEL_BYTES_EXPANDED_MATERIAL_FILTERED_WMO_COVERAGE_VERSION
                | SOURCE_EXPANDED_V2_MATERIAL_FILTERED_WMO_COVERAGE_VERSION
                | PLACEMENT_EXPANDED_MATERIAL_FILTERED_WMO_COVERAGE_VERSION
                | ASSEMBLY_EXPANDED_MATERIAL_FILTERED_WMO_COVERAGE_VERSION
                | MODEL_OUTPUT_EXPANDED_MATERIAL_FILTERED_WMO_COVERAGE_VERSION
                | GAMEOBJECT_EXPANDED_MATERIAL_FILTERED_WMO_COVERAGE_VERSION
                | GAMEOBJECT_MISSING_ROOT_COVERAGE_VERSION
                | GAMEOBJECT_ROOT_ENCRYPTED_UNAVAILABLE_COVERAGE_VERSION
        ) {
            writer.bit(self.missing_wdt_policy == run::MissingWdtPolicy::DeclareRootAbsent)?;
        }
        writer.text(match version {
            GAMEOBJECT_EXPANDED_MATERIAL_FILTERED_WMO_COVERAGE_VERSION => {
                crate::plan::GAMEOBJECT_EXPANDED_MATERIAL_FILTERED_WMO_GAMEOBJECT_SCOPE
            }
            GAMEOBJECT_MISSING_ROOT_COVERAGE_VERSION => {
                crate::plan::GAMEOBJECT_MISSING_ROOT_MATERIAL_FILTERED_WMO_GAMEOBJECT_SCOPE
            }
            GAMEOBJECT_ROOT_ENCRYPTED_UNAVAILABLE_COVERAGE_VERSION => {
                crate::plan::GAMEOBJECT_ROOT_ENCRYPTED_UNAVAILABLE_MATERIAL_FILTERED_WMO_GAMEOBJECT_SCOPE
            }
            _ => crate::plan::SELECTED_GEOMETRY_GAMEOBJECT_SCOPE,
        })?;
        writer.text(self.model_scope.profile())?;
        writer.text("available-section-map-partial-scope-v1")?;
        writer.bit(false)?; // No complete Map-source claim.
        writer.text(match version {
            GAMEOBJECT_EXPANDED_MATERIAL_FILTERED_WMO_COVERAGE_VERSION => GAMEOBJECT_SCOPE_PROFILE,
            GAMEOBJECT_MISSING_ROOT_COVERAGE_VERSION => GAMEOBJECT_MISSING_ROOT_SCOPE_PROFILE,
            GAMEOBJECT_ROOT_ENCRYPTED_UNAVAILABLE_COVERAGE_VERSION => {
                GAMEOBJECT_ROOT_ENCRYPTED_UNAVAILABLE_SCOPE_PROFILE
            }
            _ => self.gameobject_scope.as_str(),
        })?;
        writer.text(
            if matches!(
                version,
                GAMEOBJECT_EXPANDED_MATERIAL_FILTERED_WMO_COVERAGE_VERSION
                    | GAMEOBJECT_MISSING_ROOT_COVERAGE_VERSION
                    | GAMEOBJECT_ROOT_ENCRYPTED_UNAVAILABLE_COVERAGE_VERSION
            ) {
                "none"
            } else {
                self.gameobject_block_reason.as_str()
            },
        )?;
        writer.bit(
            if matches!(
                version,
                GAMEOBJECT_EXPANDED_MATERIAL_FILTERED_WMO_COVERAGE_VERSION
                    | GAMEOBJECT_MISSING_ROOT_COVERAGE_VERSION
                    | GAMEOBJECT_ROOT_ENCRYPTED_UNAVAILABLE_COVERAGE_VERSION
            ) {
                true
            } else {
                self.gameobject_request_made
            },
        )?;
        writer.text(self.model_scope.as_str())?;
        writer.text("complete-LiquidMaterial-LiquidType-v1")?;
        writer.text(self.liquid_object_scope.as_str())?;
        writer.bit(self.liquid_object_request_made)?;
        writer.bit(false)?; // No complete global-map-coverage claim.
        writer.u32(self.gameobject_file_data_id)?;
        writer.u64(
            if matches!(
                version,
                GAMEOBJECT_EXPANDED_MATERIAL_FILTERED_WMO_COVERAGE_VERSION
                    | GAMEOBJECT_MISSING_ROOT_COVERAGE_VERSION
                    | GAMEOBJECT_ROOT_ENCRYPTED_UNAVAILABLE_COVERAGE_VERSION
            ) {
                self.gameobject_startup_reads
            } else {
                0
            },
        )?;
        // Declared facts: available-section Map, complete liquid tables,
        // LiquidObject out of scope, and no global-map coverage claim.
        writer.u32(COVERAGE_FACTS)?;
        writer.u32(self.map.file_data_id)?;
        writer.u64(self.map.declared_bytes)?;
        writer.u64(self.map.available_bytes)?;
        writer.bit(self.map.full_ckey_verified)?;
        writer.bit(self.map.encoded_key_authenticated)?;
        for value in [
            self.map.declared_direct_rows,
            self.map.declared_copy_records,
            self.map.direct_rows,
            self.map.copy_records,
            self.map.materialized_rows,
        ] {
            writer.u64(value)?;
        }
        writer.u32(u32::try_from(self.map.sections.len()).map_err(|_| LIMIT)?)?;
        for section in &self.map.sections {
            writer.u32(section.ordinal)?;
            writer.u32(section.records)?;
            writer.u32(section.copies)?;
            writer.bit(section.admitted)?;
        }
        writer.u32(2)?;
        for liquid in self.liquids {
            writer.u32(liquid.file_data_id)?;
            writer.u64(liquid.bytes)?;
            writer.put(&liquid.magic)?;
            writer.u64(liquid.direct_rows)?;
            writer.u64(liquid.copy_records)?;
            writer.u64(liquid.materialized_rows)?;
        }
        writer.u32(u32::try_from(self.maps.len()).map_err(|_| LIMIT)?)?;
        for map in &self.maps {
            writer.u32(map.id)?;
            writer.bit(map.parent.is_some())?;
            writer.u32(map.parent.unwrap_or(0))?;
            writer.u32(map.wdt_file_data_id)?;
            writer.u32(map.depth)?;
            writer.bit(map.is_parent)?;
            if matches!(
                version,
                ROOT_ABSENT_COVERAGE_VERSION
                    | MODEL_ROOT_ABSENT_COVERAGE_VERSION
                    | REQUEST_CAPACITY_COVERAGE_VERSION
                    | VACATED_WMO_COVERAGE_VERSION
                    | EMPTY_WMO_COVERAGE_VERSION
                    | UNSELECTED_WMO_COVERAGE_VERSION
                    | MATERIAL_FILTERED_WMO_COVERAGE_VERSION
                    | DEPENDENCY_HEAVY_MATERIAL_FILTERED_WMO_COVERAGE_VERSION
                    | SOURCE_HEAVY_MATERIAL_FILTERED_WMO_COVERAGE_VERSION
                    | READ_HEAVY_MATERIAL_FILTERED_WMO_COVERAGE_VERSION
                    | MODEL_HEAVY_MATERIAL_FILTERED_WMO_COVERAGE_VERSION
                    | SOURCE_EXPANDED_MATERIAL_FILTERED_WMO_COVERAGE_VERSION
                    | MODEL_BYTES_EXPANDED_MATERIAL_FILTERED_WMO_COVERAGE_VERSION
                    | SOURCE_EXPANDED_V2_MATERIAL_FILTERED_WMO_COVERAGE_VERSION
                    | PLACEMENT_EXPANDED_MATERIAL_FILTERED_WMO_COVERAGE_VERSION
                    | ASSEMBLY_EXPANDED_MATERIAL_FILTERED_WMO_COVERAGE_VERSION
                    | MODEL_OUTPUT_EXPANDED_MATERIAL_FILTERED_WMO_COVERAGE_VERSION
                    | GAMEOBJECT_EXPANDED_MATERIAL_FILTERED_WMO_COVERAGE_VERSION
                    | GAMEOBJECT_MISSING_ROOT_COVERAGE_VERSION
                    | GAMEOBJECT_ROOT_ENCRYPTED_UNAVAILABLE_COVERAGE_VERSION
            ) {
                writer.bit(map.root_absent)?;
            }
        }
        if matches!(
            version,
            MODEL_ROOT_ABSENT_COVERAGE_VERSION
                | REQUEST_CAPACITY_COVERAGE_VERSION
                | VACATED_WMO_COVERAGE_VERSION
                | EMPTY_WMO_COVERAGE_VERSION
                | UNSELECTED_WMO_COVERAGE_VERSION
                | MATERIAL_FILTERED_WMO_COVERAGE_VERSION
                | DEPENDENCY_HEAVY_MATERIAL_FILTERED_WMO_COVERAGE_VERSION
                | SOURCE_HEAVY_MATERIAL_FILTERED_WMO_COVERAGE_VERSION
                | READ_HEAVY_MATERIAL_FILTERED_WMO_COVERAGE_VERSION
                | MODEL_HEAVY_MATERIAL_FILTERED_WMO_COVERAGE_VERSION
                | SOURCE_EXPANDED_MATERIAL_FILTERED_WMO_COVERAGE_VERSION
                | MODEL_BYTES_EXPANDED_MATERIAL_FILTERED_WMO_COVERAGE_VERSION
                | SOURCE_EXPANDED_V2_MATERIAL_FILTERED_WMO_COVERAGE_VERSION
                | PLACEMENT_EXPANDED_MATERIAL_FILTERED_WMO_COVERAGE_VERSION
                | ASSEMBLY_EXPANDED_MATERIAL_FILTERED_WMO_COVERAGE_VERSION
                | MODEL_OUTPUT_EXPANDED_MATERIAL_FILTERED_WMO_COVERAGE_VERSION
                | GAMEOBJECT_EXPANDED_MATERIAL_FILTERED_WMO_COVERAGE_VERSION
                | GAMEOBJECT_MISSING_ROOT_COVERAGE_VERSION
                | GAMEOBJECT_ROOT_ENCRYPTED_UNAVAILABLE_COVERAGE_VERSION
        ) {
            writer.bit(
                self.missing_m2_policy == run::MissingModelPolicy::DeclareRootAbsentWmoDoodadM2,
            )?;
            writer.u32(u32::try_from(self.missing_m2.len()).map_err(|_| LIMIT)?)?;
            for &id in &self.missing_m2 {
                writer.u32(id)?;
            }
        }
        if matches!(
            version,
            REQUEST_CAPACITY_COVERAGE_VERSION
                | VACATED_WMO_COVERAGE_VERSION
                | EMPTY_WMO_COVERAGE_VERSION
                | UNSELECTED_WMO_COVERAGE_VERSION
                | MATERIAL_FILTERED_WMO_COVERAGE_VERSION
                | DEPENDENCY_HEAVY_MATERIAL_FILTERED_WMO_COVERAGE_VERSION
                | SOURCE_HEAVY_MATERIAL_FILTERED_WMO_COVERAGE_VERSION
                | READ_HEAVY_MATERIAL_FILTERED_WMO_COVERAGE_VERSION
                | MODEL_HEAVY_MATERIAL_FILTERED_WMO_COVERAGE_VERSION
                | SOURCE_EXPANDED_MATERIAL_FILTERED_WMO_COVERAGE_VERSION
                | MODEL_BYTES_EXPANDED_MATERIAL_FILTERED_WMO_COVERAGE_VERSION
                | SOURCE_EXPANDED_V2_MATERIAL_FILTERED_WMO_COVERAGE_VERSION
                | PLACEMENT_EXPANDED_MATERIAL_FILTERED_WMO_COVERAGE_VERSION
                | ASSEMBLY_EXPANDED_MATERIAL_FILTERED_WMO_COVERAGE_VERSION
                | MODEL_OUTPUT_EXPANDED_MATERIAL_FILTERED_WMO_COVERAGE_VERSION
                | GAMEOBJECT_EXPANDED_MATERIAL_FILTERED_WMO_COVERAGE_VERSION
                | GAMEOBJECT_MISSING_ROOT_COVERAGE_VERSION
                | GAMEOBJECT_ROOT_ENCRYPTED_UNAVAILABLE_COVERAGE_VERSION
        ) {
            writer.text(self.resource_profile.id())?;
        }
        if matches!(
            version,
            VACATED_WMO_COVERAGE_VERSION
                | EMPTY_WMO_COVERAGE_VERSION
                | UNSELECTED_WMO_COVERAGE_VERSION
                | MATERIAL_FILTERED_WMO_COVERAGE_VERSION
                | DEPENDENCY_HEAVY_MATERIAL_FILTERED_WMO_COVERAGE_VERSION
                | SOURCE_HEAVY_MATERIAL_FILTERED_WMO_COVERAGE_VERSION
                | READ_HEAVY_MATERIAL_FILTERED_WMO_COVERAGE_VERSION
                | MODEL_HEAVY_MATERIAL_FILTERED_WMO_COVERAGE_VERSION
                | SOURCE_EXPANDED_MATERIAL_FILTERED_WMO_COVERAGE_VERSION
                | MODEL_BYTES_EXPANDED_MATERIAL_FILTERED_WMO_COVERAGE_VERSION
                | SOURCE_EXPANDED_V2_MATERIAL_FILTERED_WMO_COVERAGE_VERSION
                | PLACEMENT_EXPANDED_MATERIAL_FILTERED_WMO_COVERAGE_VERSION
                | ASSEMBLY_EXPANDED_MATERIAL_FILTERED_WMO_COVERAGE_VERSION
                | MODEL_OUTPUT_EXPANDED_MATERIAL_FILTERED_WMO_COVERAGE_VERSION
                | GAMEOBJECT_EXPANDED_MATERIAL_FILTERED_WMO_COVERAGE_VERSION
                | GAMEOBJECT_MISSING_ROOT_COVERAGE_VERSION
                | GAMEOBJECT_ROOT_ENCRYPTED_UNAVAILABLE_COVERAGE_VERSION
        ) {
            writer.bit(matches!(
                self.missing_wmo_policy,
                run::MissingWmoPolicy::DeclareVacatedAfterRootAbsentM2
                    | run::MissingWmoPolicy::DeclareVacatedAfterRootAbsentM2OrEmpty
                    | run::MissingWmoPolicy::DeclareVacatedAfterRootAbsentM2OrEmptyOrUnselected
                    | run::MissingWmoPolicy::DeclareVacatedAfterRootAbsentM2OrEmptyOrUnselectedOrMaterialFiltered
            ))?;
            writer.u32(u32::try_from(self.missing_wmo.len()).map_err(|_| LIMIT)?)?;
            for &id in &self.missing_wmo {
                writer.u32(id)?;
            }
        }
        if matches!(
            version,
            EMPTY_WMO_COVERAGE_VERSION
                | UNSELECTED_WMO_COVERAGE_VERSION
                | MATERIAL_FILTERED_WMO_COVERAGE_VERSION
                | DEPENDENCY_HEAVY_MATERIAL_FILTERED_WMO_COVERAGE_VERSION
                | SOURCE_HEAVY_MATERIAL_FILTERED_WMO_COVERAGE_VERSION
                | READ_HEAVY_MATERIAL_FILTERED_WMO_COVERAGE_VERSION
                | MODEL_HEAVY_MATERIAL_FILTERED_WMO_COVERAGE_VERSION
                | SOURCE_EXPANDED_MATERIAL_FILTERED_WMO_COVERAGE_VERSION
                | MODEL_BYTES_EXPANDED_MATERIAL_FILTERED_WMO_COVERAGE_VERSION
                | SOURCE_EXPANDED_V2_MATERIAL_FILTERED_WMO_COVERAGE_VERSION
                | PLACEMENT_EXPANDED_MATERIAL_FILTERED_WMO_COVERAGE_VERSION
                | ASSEMBLY_EXPANDED_MATERIAL_FILTERED_WMO_COVERAGE_VERSION
                | MODEL_OUTPUT_EXPANDED_MATERIAL_FILTERED_WMO_COVERAGE_VERSION
                | GAMEOBJECT_EXPANDED_MATERIAL_FILTERED_WMO_COVERAGE_VERSION
                | GAMEOBJECT_MISSING_ROOT_COVERAGE_VERSION
                | GAMEOBJECT_ROOT_ENCRYPTED_UNAVAILABLE_COVERAGE_VERSION
        ) {
            writer.u32(u32::try_from(self.empty_wmo.len()).map_err(|_| LIMIT)?)?;
            for &id in &self.empty_wmo {
                writer.u32(id)?;
            }
        }
        if matches!(
            version,
            UNSELECTED_WMO_COVERAGE_VERSION
                | MATERIAL_FILTERED_WMO_COVERAGE_VERSION
                | DEPENDENCY_HEAVY_MATERIAL_FILTERED_WMO_COVERAGE_VERSION
                | SOURCE_HEAVY_MATERIAL_FILTERED_WMO_COVERAGE_VERSION
                | READ_HEAVY_MATERIAL_FILTERED_WMO_COVERAGE_VERSION
                | MODEL_HEAVY_MATERIAL_FILTERED_WMO_COVERAGE_VERSION
                | SOURCE_EXPANDED_MATERIAL_FILTERED_WMO_COVERAGE_VERSION
                | MODEL_BYTES_EXPANDED_MATERIAL_FILTERED_WMO_COVERAGE_VERSION
                | SOURCE_EXPANDED_V2_MATERIAL_FILTERED_WMO_COVERAGE_VERSION
                | PLACEMENT_EXPANDED_MATERIAL_FILTERED_WMO_COVERAGE_VERSION
                | ASSEMBLY_EXPANDED_MATERIAL_FILTERED_WMO_COVERAGE_VERSION
                | MODEL_OUTPUT_EXPANDED_MATERIAL_FILTERED_WMO_COVERAGE_VERSION
                | GAMEOBJECT_EXPANDED_MATERIAL_FILTERED_WMO_COVERAGE_VERSION
                | GAMEOBJECT_MISSING_ROOT_COVERAGE_VERSION
                | GAMEOBJECT_ROOT_ENCRYPTED_UNAVAILABLE_COVERAGE_VERSION
        ) {
            writer.u32(u32::try_from(self.unselected_wmo.len()).map_err(|_| LIMIT)?)?;
            for &id in &self.unselected_wmo {
                writer.u32(id)?;
            }
        }
        if matches!(
            version,
            MATERIAL_FILTERED_WMO_COVERAGE_VERSION
                | DEPENDENCY_HEAVY_MATERIAL_FILTERED_WMO_COVERAGE_VERSION
                | SOURCE_HEAVY_MATERIAL_FILTERED_WMO_COVERAGE_VERSION
                | READ_HEAVY_MATERIAL_FILTERED_WMO_COVERAGE_VERSION
                | MODEL_HEAVY_MATERIAL_FILTERED_WMO_COVERAGE_VERSION
                | SOURCE_EXPANDED_MATERIAL_FILTERED_WMO_COVERAGE_VERSION
                | MODEL_BYTES_EXPANDED_MATERIAL_FILTERED_WMO_COVERAGE_VERSION
                | SOURCE_EXPANDED_V2_MATERIAL_FILTERED_WMO_COVERAGE_VERSION
                | PLACEMENT_EXPANDED_MATERIAL_FILTERED_WMO_COVERAGE_VERSION
                | ASSEMBLY_EXPANDED_MATERIAL_FILTERED_WMO_COVERAGE_VERSION
                | MODEL_OUTPUT_EXPANDED_MATERIAL_FILTERED_WMO_COVERAGE_VERSION
                | GAMEOBJECT_EXPANDED_MATERIAL_FILTERED_WMO_COVERAGE_VERSION
                | GAMEOBJECT_MISSING_ROOT_COVERAGE_VERSION
                | GAMEOBJECT_ROOT_ENCRYPTED_UNAVAILABLE_COVERAGE_VERSION
        ) {
            writer.u32(u32::try_from(self.material_filtered_wmo.len()).map_err(|_| LIMIT)?)?;
            for &id in &self.material_filtered_wmo {
                writer.u32(id)?;
            }
        }
        if matches!(
            version,
            GAMEOBJECT_EXPANDED_MATERIAL_FILTERED_WMO_COVERAGE_VERSION
                | GAMEOBJECT_MISSING_ROOT_COVERAGE_VERSION
                | GAMEOBJECT_ROOT_ENCRYPTED_UNAVAILABLE_COVERAGE_VERSION
        ) {
            let facts = self.gameobjects.as_ref().ok_or(SCHEMA)?;
            encode_gameobject_facts(
                &mut writer,
                facts,
                version != GAMEOBJECT_EXPANDED_MATERIAL_FILTERED_WMO_COVERAGE_VERSION,
                version == GAMEOBJECT_ROOT_ENCRYPTED_UNAVAILABLE_COVERAGE_VERSION,
            )?;
        } else if self.gameobjects.is_some() {
            return Err(SCHEMA.into());
        }
        writer.u32(u32::try_from(self.artifacts.len()).map_err(|_| LIMIT)?)?;
        for artifact in &self.artifacts {
            writer.text(&artifact.path)?;
            writer.u64(artifact.bytes)?;
            writer.put(&artifact.sha)?;
        }
        Ok(writer.bytes)
    }

    pub(crate) fn parse(bytes: &[u8]) -> Result<Self, Error> {
        Self::parse_with_components(bytes, Component::Collision.bit(), COVERAGE_VERSION)
    }

    pub(crate) fn parse_v28(bytes: &[u8]) -> Result<Self, Error> {
        Self::parse_with_components(
            bytes,
            Component::Db2.bit() | Component::Collision.bit(),
            COVERAGE_VERSION,
        )
    }

    pub(crate) fn parse_v29(bytes: &[u8]) -> Result<Self, Error> {
        Self::parse_with_components(
            bytes,
            Component::Db2.bit() | Component::Collision.bit(),
            AVAILABLE_COVERAGE_VERSION,
        )
    }

    pub(crate) fn parse_v30(bytes: &[u8]) -> Result<Self, Error> {
        Self::parse_with_components(
            bytes,
            Component::Db2.bit() | Component::Collision.bit(),
            ROOT_ABSENT_COVERAGE_VERSION,
        )
    }

    pub(crate) fn parse_v31(bytes: &[u8]) -> Result<Self, Error> {
        Self::parse_with_components(
            bytes,
            Component::Db2.bit() | Component::Collision.bit(),
            MODEL_ROOT_ABSENT_COVERAGE_VERSION,
        )
    }

    pub(crate) fn parse_v32(bytes: &[u8]) -> Result<Self, Error> {
        Self::parse_with_components(
            bytes,
            Component::Db2.bit() | Component::Collision.bit(),
            REQUEST_CAPACITY_COVERAGE_VERSION,
        )
    }

    pub(crate) fn parse_v33(bytes: &[u8]) -> Result<Self, Error> {
        Self::parse_with_components(
            bytes,
            Component::Db2.bit() | Component::Collision.bit(),
            VACATED_WMO_COVERAGE_VERSION,
        )
    }

    pub(crate) fn parse_v34(bytes: &[u8]) -> Result<Self, Error> {
        Self::parse_with_components(
            bytes,
            Component::Db2.bit() | Component::Collision.bit(),
            EMPTY_WMO_COVERAGE_VERSION,
        )
    }

    pub(crate) fn parse_v35(bytes: &[u8]) -> Result<Self, Error> {
        Self::parse_with_components(
            bytes,
            Component::Db2.bit() | Component::Collision.bit(),
            UNSELECTED_WMO_COVERAGE_VERSION,
        )
    }

    pub(crate) fn parse_v36(bytes: &[u8]) -> Result<Self, Error> {
        Self::parse_with_components(
            bytes,
            Component::Db2.bit() | Component::Collision.bit(),
            MATERIAL_FILTERED_WMO_COVERAGE_VERSION,
        )
    }

    pub(crate) fn parse_v37(bytes: &[u8]) -> Result<Self, Error> {
        Self::parse_with_components(
            bytes,
            Component::Db2.bit() | Component::Collision.bit(),
            DEPENDENCY_HEAVY_MATERIAL_FILTERED_WMO_COVERAGE_VERSION,
        )
    }

    pub(crate) fn parse_v38(bytes: &[u8]) -> Result<Self, Error> {
        Self::parse_with_components(
            bytes,
            Component::Db2.bit() | Component::Collision.bit(),
            SOURCE_HEAVY_MATERIAL_FILTERED_WMO_COVERAGE_VERSION,
        )
    }

    pub(crate) fn parse_v39(bytes: &[u8]) -> Result<Self, Error> {
        Self::parse_with_components(
            bytes,
            Component::Db2.bit() | Component::Collision.bit(),
            READ_HEAVY_MATERIAL_FILTERED_WMO_COVERAGE_VERSION,
        )
    }

    pub(crate) fn parse_v40(bytes: &[u8]) -> Result<Self, Error> {
        Self::parse_with_components(
            bytes,
            Component::Db2.bit() | Component::Collision.bit(),
            MODEL_HEAVY_MATERIAL_FILTERED_WMO_COVERAGE_VERSION,
        )
    }

    pub(crate) fn parse_v41(bytes: &[u8]) -> Result<Self, Error> {
        Self::parse_with_components(
            bytes,
            Component::Db2.bit() | Component::Collision.bit(),
            SOURCE_EXPANDED_MATERIAL_FILTERED_WMO_COVERAGE_VERSION,
        )
    }

    pub(crate) fn parse_v42(bytes: &[u8]) -> Result<Self, Error> {
        Self::parse_with_components(
            bytes,
            Component::Db2.bit() | Component::Collision.bit(),
            MODEL_OUTPUT_EXPANDED_MATERIAL_FILTERED_WMO_COVERAGE_VERSION,
        )
    }

    pub(crate) fn parse_v43(bytes: &[u8]) -> Result<Self, Error> {
        Self::parse_with_components(
            bytes,
            Component::Db2.bit() | Component::Collision.bit(),
            MODEL_BYTES_EXPANDED_MATERIAL_FILTERED_WMO_COVERAGE_VERSION,
        )
    }

    pub(crate) fn parse_v44(bytes: &[u8]) -> Result<Self, Error> {
        Self::parse_with_components(
            bytes,
            Component::Db2.bit() | Component::Collision.bit(),
            SOURCE_EXPANDED_V2_MATERIAL_FILTERED_WMO_COVERAGE_VERSION,
        )
    }

    pub(crate) fn parse_v45(bytes: &[u8]) -> Result<Self, Error> {
        Self::parse_with_components(
            bytes,
            Component::Db2.bit() | Component::Collision.bit(),
            PLACEMENT_EXPANDED_MATERIAL_FILTERED_WMO_COVERAGE_VERSION,
        )
    }

    pub(crate) fn parse_v46(bytes: &[u8]) -> Result<Self, Error> {
        Self::parse_with_components(
            bytes,
            Component::Db2.bit() | Component::Collision.bit(),
            ASSEMBLY_EXPANDED_MATERIAL_FILTERED_WMO_COVERAGE_VERSION,
        )
    }

    pub(crate) fn parse_v47(bytes: &[u8]) -> Result<Self, Error> {
        Self::parse_with_components(
            bytes,
            Component::Db2.bit() | Component::Collision.bit(),
            GAMEOBJECT_EXPANDED_MATERIAL_FILTERED_WMO_COVERAGE_VERSION,
        )
    }

    pub(crate) fn parse_v48(bytes: &[u8]) -> Result<Self, Error> {
        Self::parse_with_components(
            bytes,
            Component::Db2.bit() | Component::Collision.bit(),
            GAMEOBJECT_MISSING_ROOT_COVERAGE_VERSION,
        )
    }

    pub(crate) fn parse_v49(bytes: &[u8]) -> Result<Self, Error> {
        Self::parse_with_components(
            bytes,
            Component::Db2.bit() | Component::Collision.bit(),
            GAMEOBJECT_ROOT_ENCRYPTED_UNAVAILABLE_COVERAGE_VERSION,
        )
    }

    pub(crate) fn parse_v51(bytes: &[u8]) -> Result<Self, Error> {
        Self::parse_with_components(
            bytes,
            25,
            GAMEOBJECT_ROOT_ENCRYPTED_UNAVAILABLE_COVERAGE_VERSION,
        )
    }

    fn parse_with_components(
        bytes: &[u8],
        expected_components: u8,
        expected_version: u32,
    ) -> Result<Self, Error> {
        if bytes.len() > COVERAGE_MAX_BYTES {
            return Err(LIMIT.into());
        }
        let mut reader = Reader { bytes, at: 0 };
        if reader.take(8)? != COVERAGE_MAGIC {
            return Err(SCHEMA.into());
        }
        let version = reader.u32()?;
        if version != expected_version {
            return Err(SCHEMA.into());
        }
        let context = Context::parse(reader.text()?)?;
        if context.build != 70450 || context.components != expected_components {
            return Err(SCHEMA.into());
        }
        let (selection, coverage_profile, profile, domain_profile) = match version {
            COVERAGE_VERSION => (
                MapSelection::MapAndAncestors(reader.u32()?),
                crate::plan::SELECTED_GEOMETRY_COVERAGE_PROFILE,
                PROFILE,
                "available-map-active-ancestors-v1",
            ),
            AVAILABLE_COVERAGE_VERSION => (
                MapSelection::All,
                crate::plan::AVAILABLE_GEOMETRY_COVERAGE_PROFILE,
                AVAILABLE_COVERAGE_PROFILE,
                "available-map-all-active-v1",
            ),
            ROOT_ABSENT_COVERAGE_VERSION => (
                MapSelection::All,
                crate::plan::ROOT_ABSENT_GEOMETRY_COVERAGE_PROFILE,
                ROOT_ABSENT_COVERAGE_PROFILE,
                ROOT_ABSENT_DOMAIN_PROFILE,
            ),
            MODEL_ROOT_ABSENT_COVERAGE_VERSION => (
                MapSelection::All,
                crate::plan::MODEL_ROOT_ABSENT_GEOMETRY_COVERAGE_PROFILE,
                MODEL_ROOT_ABSENT_COVERAGE_PROFILE,
                MODEL_ROOT_ABSENT_DOMAIN_PROFILE,
            ),
            REQUEST_CAPACITY_COVERAGE_VERSION => (
                MapSelection::All,
                crate::plan::MODEL_ROOT_ABSENT_REQUEST_CAPACITY_GEOMETRY_COVERAGE_PROFILE,
                REQUEST_CAPACITY_COVERAGE_PROFILE,
                REQUEST_CAPACITY_DOMAIN_PROFILE,
            ),
            VACATED_WMO_COVERAGE_VERSION => (
                MapSelection::All,
                crate::plan::VACATED_WMO_COVERAGE_PROFILE,
                VACATED_WMO_CODEC_PROFILE,
                VACATED_WMO_DOMAIN_PROFILE,
            ),
            EMPTY_WMO_COVERAGE_VERSION => (
                MapSelection::All,
                crate::plan::EMPTY_WMO_COVERAGE_PROFILE,
                EMPTY_WMO_CODEC_PROFILE,
                EMPTY_WMO_DOMAIN_PROFILE,
            ),
            UNSELECTED_WMO_COVERAGE_VERSION => (
                MapSelection::All,
                crate::plan::UNSELECTED_WMO_COVERAGE_PROFILE,
                UNSELECTED_WMO_CODEC_PROFILE,
                UNSELECTED_WMO_DOMAIN_PROFILE,
            ),
            MATERIAL_FILTERED_WMO_COVERAGE_VERSION => (
                MapSelection::All,
                crate::plan::MATERIAL_FILTERED_WMO_COVERAGE_PROFILE,
                MATERIAL_FILTERED_WMO_CODEC_PROFILE,
                MATERIAL_FILTERED_WMO_DOMAIN_PROFILE,
            ),
            DEPENDENCY_HEAVY_MATERIAL_FILTERED_WMO_COVERAGE_VERSION => (
                MapSelection::All,
                crate::plan::DEPENDENCY_HEAVY_MATERIAL_FILTERED_WMO_COVERAGE_PROFILE,
                DEPENDENCY_HEAVY_MATERIAL_FILTERED_WMO_CODEC_PROFILE,
                DEPENDENCY_HEAVY_MATERIAL_FILTERED_WMO_DOMAIN_PROFILE,
            ),
            SOURCE_HEAVY_MATERIAL_FILTERED_WMO_COVERAGE_VERSION => (
                MapSelection::All,
                crate::plan::SOURCE_HEAVY_MATERIAL_FILTERED_WMO_COVERAGE_PROFILE,
                SOURCE_HEAVY_MATERIAL_FILTERED_WMO_CODEC_PROFILE,
                SOURCE_HEAVY_MATERIAL_FILTERED_WMO_DOMAIN_PROFILE,
            ),
            READ_HEAVY_MATERIAL_FILTERED_WMO_COVERAGE_VERSION => (
                MapSelection::All,
                crate::plan::READ_HEAVY_MATERIAL_FILTERED_WMO_COVERAGE_PROFILE,
                READ_HEAVY_MATERIAL_FILTERED_WMO_CODEC_PROFILE,
                READ_HEAVY_MATERIAL_FILTERED_WMO_DOMAIN_PROFILE,
            ),
            MODEL_HEAVY_MATERIAL_FILTERED_WMO_COVERAGE_VERSION => (
                MapSelection::All,
                crate::plan::MODEL_HEAVY_MATERIAL_FILTERED_WMO_COVERAGE_PROFILE,
                MODEL_HEAVY_MATERIAL_FILTERED_WMO_CODEC_PROFILE,
                MODEL_HEAVY_MATERIAL_FILTERED_WMO_DOMAIN_PROFILE,
            ),
            SOURCE_EXPANDED_MATERIAL_FILTERED_WMO_COVERAGE_VERSION => (
                MapSelection::All,
                crate::plan::SOURCE_EXPANDED_MATERIAL_FILTERED_WMO_COVERAGE_PROFILE,
                SOURCE_EXPANDED_MATERIAL_FILTERED_WMO_CODEC_PROFILE,
                SOURCE_EXPANDED_MATERIAL_FILTERED_WMO_DOMAIN_PROFILE,
            ),
            MODEL_OUTPUT_EXPANDED_MATERIAL_FILTERED_WMO_COVERAGE_VERSION => (
                MapSelection::All,
                crate::plan::MODEL_OUTPUT_EXPANDED_MATERIAL_FILTERED_WMO_COVERAGE_PROFILE,
                MODEL_OUTPUT_EXPANDED_MATERIAL_FILTERED_WMO_CODEC_PROFILE,
                MODEL_OUTPUT_EXPANDED_MATERIAL_FILTERED_WMO_DOMAIN_PROFILE,
            ),
            MODEL_BYTES_EXPANDED_MATERIAL_FILTERED_WMO_COVERAGE_VERSION => (
                MapSelection::All,
                crate::plan::MODEL_BYTES_EXPANDED_MATERIAL_FILTERED_WMO_COVERAGE_PROFILE,
                MODEL_BYTES_EXPANDED_MATERIAL_FILTERED_WMO_CODEC_PROFILE,
                MODEL_BYTES_EXPANDED_MATERIAL_FILTERED_WMO_DOMAIN_PROFILE,
            ),
            SOURCE_EXPANDED_V2_MATERIAL_FILTERED_WMO_COVERAGE_VERSION => (
                MapSelection::All,
                crate::plan::SOURCE_EXPANDED_V2_MATERIAL_FILTERED_WMO_COVERAGE_PROFILE,
                SOURCE_EXPANDED_V2_MATERIAL_FILTERED_WMO_CODEC_PROFILE,
                SOURCE_EXPANDED_V2_MATERIAL_FILTERED_WMO_DOMAIN_PROFILE,
            ),
            PLACEMENT_EXPANDED_MATERIAL_FILTERED_WMO_COVERAGE_VERSION => (
                MapSelection::All,
                crate::plan::PLACEMENT_EXPANDED_MATERIAL_FILTERED_WMO_COVERAGE_PROFILE,
                PLACEMENT_EXPANDED_MATERIAL_FILTERED_WMO_CODEC_PROFILE,
                PLACEMENT_EXPANDED_MATERIAL_FILTERED_WMO_DOMAIN_PROFILE,
            ),
            ASSEMBLY_EXPANDED_MATERIAL_FILTERED_WMO_COVERAGE_VERSION => (
                MapSelection::All,
                crate::plan::ASSEMBLY_EXPANDED_MATERIAL_FILTERED_WMO_COVERAGE_PROFILE,
                ASSEMBLY_EXPANDED_MATERIAL_FILTERED_WMO_CODEC_PROFILE,
                ASSEMBLY_EXPANDED_MATERIAL_FILTERED_WMO_DOMAIN_PROFILE,
            ),
            GAMEOBJECT_EXPANDED_MATERIAL_FILTERED_WMO_COVERAGE_VERSION => (
                MapSelection::All,
                crate::plan::GAMEOBJECT_EXPANDED_MATERIAL_FILTERED_WMO_COVERAGE_PROFILE,
                GAMEOBJECT_EXPANDED_MATERIAL_FILTERED_WMO_CODEC_PROFILE,
                GAMEOBJECT_EXPANDED_MATERIAL_FILTERED_WMO_DOMAIN_PROFILE,
            ),
            GAMEOBJECT_MISSING_ROOT_COVERAGE_VERSION => (
                MapSelection::All,
                GAMEOBJECT_MISSING_ROOT_COVERAGE_PROFILE,
                GAMEOBJECT_MISSING_ROOT_CODEC_PROFILE,
                GAMEOBJECT_MISSING_ROOT_DOMAIN_PROFILE,
            ),
            GAMEOBJECT_ROOT_ENCRYPTED_UNAVAILABLE_COVERAGE_VERSION if expected_components == 25 => {
                (
                    MapSelection::All,
                    crate::plan::DB2_TERRAIN_LIQUID_VMAP_COVERAGE_PROFILE,
                    DB2_TERRAIN_LIQUID_VMAP_CODEC_PROFILE,
                    crate::plan::DB2_TERRAIN_LIQUID_VMAP_DOMAIN_PROFILE,
                )
            }
            GAMEOBJECT_ROOT_ENCRYPTED_UNAVAILABLE_COVERAGE_VERSION => (
                MapSelection::All,
                GAMEOBJECT_ROOT_ENCRYPTED_UNAVAILABLE_COVERAGE_PROFILE,
                GAMEOBJECT_ROOT_ENCRYPTED_UNAVAILABLE_CODEC_PROFILE,
                GAMEOBJECT_ROOT_ENCRYPTED_UNAVAILABLE_DOMAIN_PROFILE,
            ),
            _ => return Err(SCHEMA.into()),
        };
        if reader.text()? != coverage_profile
            || reader.text()? != profile
            || reader.text()? != domain_profile
        {
            return Err(SCHEMA.into());
        }
        let missing_wdt_policy = if matches!(
            version,
            ROOT_ABSENT_COVERAGE_VERSION
                | MODEL_ROOT_ABSENT_COVERAGE_VERSION
                | REQUEST_CAPACITY_COVERAGE_VERSION
                | VACATED_WMO_COVERAGE_VERSION
                | EMPTY_WMO_COVERAGE_VERSION
                | UNSELECTED_WMO_COVERAGE_VERSION
                | MATERIAL_FILTERED_WMO_COVERAGE_VERSION
                | DEPENDENCY_HEAVY_MATERIAL_FILTERED_WMO_COVERAGE_VERSION
                | SOURCE_HEAVY_MATERIAL_FILTERED_WMO_COVERAGE_VERSION
                | READ_HEAVY_MATERIAL_FILTERED_WMO_COVERAGE_VERSION
                | MODEL_HEAVY_MATERIAL_FILTERED_WMO_COVERAGE_VERSION
                | SOURCE_EXPANDED_MATERIAL_FILTERED_WMO_COVERAGE_VERSION
                | MODEL_BYTES_EXPANDED_MATERIAL_FILTERED_WMO_COVERAGE_VERSION
                | SOURCE_EXPANDED_V2_MATERIAL_FILTERED_WMO_COVERAGE_VERSION
                | PLACEMENT_EXPANDED_MATERIAL_FILTERED_WMO_COVERAGE_VERSION
                | ASSEMBLY_EXPANDED_MATERIAL_FILTERED_WMO_COVERAGE_VERSION
                | MODEL_OUTPUT_EXPANDED_MATERIAL_FILTERED_WMO_COVERAGE_VERSION
                | GAMEOBJECT_EXPANDED_MATERIAL_FILTERED_WMO_COVERAGE_VERSION
                | GAMEOBJECT_MISSING_ROOT_COVERAGE_VERSION
                | GAMEOBJECT_ROOT_ENCRYPTED_UNAVAILABLE_COVERAGE_VERSION
        ) {
            if !reader.bit()? {
                return Err(SCHEMA.into());
            }
            run::MissingWdtPolicy::DeclareRootAbsent
        } else {
            run::MissingWdtPolicy::Reject
        };
        let gameobject_scope_profile = match version {
            GAMEOBJECT_EXPANDED_MATERIAL_FILTERED_WMO_COVERAGE_VERSION => {
                crate::plan::GAMEOBJECT_EXPANDED_MATERIAL_FILTERED_WMO_GAMEOBJECT_SCOPE
            }
            GAMEOBJECT_MISSING_ROOT_COVERAGE_VERSION => {
                crate::plan::GAMEOBJECT_MISSING_ROOT_MATERIAL_FILTERED_WMO_GAMEOBJECT_SCOPE
            }
            GAMEOBJECT_ROOT_ENCRYPTED_UNAVAILABLE_COVERAGE_VERSION => {
                crate::plan::GAMEOBJECT_ROOT_ENCRYPTED_UNAVAILABLE_MATERIAL_FILTERED_WMO_GAMEOBJECT_SCOPE
            }
            _ => crate::plan::SELECTED_GEOMETRY_GAMEOBJECT_SCOPE,
        };
        if reader.text()? != gameobject_scope_profile {
            return Err(SCHEMA.into());
        }
        let model_scope = match (reader.text()?, reader.text()?) {
            (MODEL_SCOPE_PROFILE, "available-section-map-partial-scope-v1") => {
                ModelScope::PlacementOnly
            }
            (GAMEOBJECT_MODEL_SCOPE_PROFILE, "available-section-map-partial-scope-v1")
                if version == GAMEOBJECT_EXPANDED_MATERIAL_FILTERED_WMO_COVERAGE_VERSION =>
            {
                ModelScope::PlacementAndGameObject
            }
            (
                GAMEOBJECT_MISSING_ROOT_MODEL_SCOPE_PROFILE,
                "available-section-map-partial-scope-v1",
            ) if version == GAMEOBJECT_MISSING_ROOT_COVERAGE_VERSION => {
                ModelScope::PlacementAndGameObjectMissingRoot
            }
            (
                GAMEOBJECT_ROOT_ENCRYPTED_UNAVAILABLE_MODEL_SCOPE_PROFILE,
                "available-section-map-partial-scope-v1",
            ) if version == GAMEOBJECT_ROOT_ENCRYPTED_UNAVAILABLE_COVERAGE_VERSION => {
                ModelScope::PlacementAndGameObjectRootAndEncryptedUnavailable
            }
            _ => return Err(SCHEMA.into()),
        };
        if reader.bit()? {
            return Err(SCHEMA.into());
        }
        let (gameobject_scope, gameobject_block_reason, gameobject_request_made) = if matches!(
            version,
            GAMEOBJECT_EXPANDED_MATERIAL_FILTERED_WMO_COVERAGE_VERSION
                | GAMEOBJECT_MISSING_ROOT_COVERAGE_VERSION
                | GAMEOBJECT_ROOT_ENCRYPTED_UNAVAILABLE_COVERAGE_VERSION
        ) {
            let expected_scope = match version {
                GAMEOBJECT_MISSING_ROOT_COVERAGE_VERSION => GAMEOBJECT_MISSING_ROOT_SCOPE_PROFILE,
                GAMEOBJECT_ROOT_ENCRYPTED_UNAVAILABLE_COVERAGE_VERSION => {
                    GAMEOBJECT_ROOT_ENCRYPTED_UNAVAILABLE_SCOPE_PROFILE
                }
                _ => GAMEOBJECT_SCOPE_PROFILE,
            };
            if reader.text()? != expected_scope || reader.text()? != "none" || !reader.bit()? {
                return Err(SCHEMA.into());
            }
            (
                match version {
                    GAMEOBJECT_MISSING_ROOT_COVERAGE_VERSION => {
                        geometry::GameObjectScope::AvailableFdidDirectMissingRootV1
                    }
                    GAMEOBJECT_ROOT_ENCRYPTED_UNAVAILABLE_COVERAGE_VERSION => {
                        geometry::GameObjectScope::AvailableFdidDirectRootAbsentOrEncryptedUnavailableV1
                    }
                    _ => geometry::GameObjectScope::AvailableFdidDirectV1,
                },
                geometry::GameObjectBlockReason::None,
                true,
            )
        } else {
            let scope = match reader.text()? {
                "excluded" => geometry::GameObjectScope::Excluded,
                _ => return Err(SCHEMA.into()),
            };
            let reason = match reader.text()? {
                "excluded_by_profile" => geometry::GameObjectBlockReason::ExcludedByProfile,
                _ => return Err(SCHEMA.into()),
            };
            (scope, reason, reader.bit()?)
        };
        if gameobject_request_made
            != matches!(
                version,
                GAMEOBJECT_EXPANDED_MATERIAL_FILTERED_WMO_COVERAGE_VERSION
                    | GAMEOBJECT_MISSING_ROOT_COVERAGE_VERSION
                    | GAMEOBJECT_ROOT_ENCRYPTED_UNAVAILABLE_COVERAGE_VERSION
            )
            || reader.text()? != model_scope.as_str()
            || reader.text()? != "complete-LiquidMaterial-LiquidType-v1"
        {
            return Err(SCHEMA.into());
        }
        let liquid_object_scope = match reader.text()? {
            "out_of_scope" => geometry::LiquidObjectScope::OutOfScope,
            _ => return Err(SCHEMA.into()),
        };
        let liquid_object_request_made = reader.bit()?;
        if liquid_object_request_made || reader.bit()? {
            return Err(SCHEMA.into());
        }
        let gameobject_file_data_id = reader.u32()?;
        let gameobject_startup_reads = reader.u64()?;
        if gameobject_file_data_id != forever_casc::collision::gameobjects::TABLE_FILE_DATA_ID {
            return Err(SCHEMA.into());
        }
        if (matches!(
            version,
            GAMEOBJECT_EXPANDED_MATERIAL_FILTERED_WMO_COVERAGE_VERSION
                | GAMEOBJECT_MISSING_ROOT_COVERAGE_VERSION
                | GAMEOBJECT_ROOT_ENCRYPTED_UNAVAILABLE_COVERAGE_VERSION
        ) && gameobject_startup_reads == 0)
            || (!matches!(
                version,
                GAMEOBJECT_EXPANDED_MATERIAL_FILTERED_WMO_COVERAGE_VERSION
                    | GAMEOBJECT_MISSING_ROOT_COVERAGE_VERSION
                    | GAMEOBJECT_ROOT_ENCRYPTED_UNAVAILABLE_COVERAGE_VERSION
            ) && gameobject_startup_reads != 0)
        {
            return Err(SCHEMA.into());
        }
        if reader.u32()? != COVERAGE_FACTS {
            return Err(SCHEMA.into());
        }
        let file_data_id = reader.u32()?;
        let declared_bytes = reader.u64()?;
        let available_bytes = reader.u64()?;
        let full_ckey_verified = reader.bit()?;
        let encoded_key_authenticated = reader.bit()?;
        let declared_direct_rows = reader.u64()?;
        let declared_copy_records = reader.u64()?;
        let direct_rows = reader.u64()?;
        let copy_records = reader.u64()?;
        let materialized_rows = reader.u64()?;
        if file_data_id != world::MAP_FILE_DATA_ID
            || declared_bytes < 204
            || available_bytes > declared_bytes
            || (full_ckey_verified && available_bytes != declared_bytes)
            || (!full_ckey_verified && !encoded_key_authenticated)
            || direct_rows > declared_direct_rows
            || copy_records > declared_copy_records
            || materialized_rows < direct_rows
            || materialized_rows > direct_rows.saturating_add(copy_records)
        {
            return Err(SCHEMA.into());
        }
        let section_count = reader.count(16)?;
        let mut sections = Vec::new();
        sections
            .try_reserve_exact(section_count)
            .map_err(|_| Error::from("product_allocation"))?;
        let mut previous = None;
        let mut section_rows = 0_u64;
        let mut section_copies = 0_u64;
        let mut admitted_rows = 0_u64;
        let mut admitted_copies = 0_u64;
        for _ in 0..section_count {
            let section = SectionFact {
                ordinal: reader.u32()?,
                records: reader.u32()?,
                copies: reader.u32()?,
                admitted: reader.bit()?,
            };
            if section.ordinal >= 16 || previous.is_some_and(|ordinal| ordinal >= section.ordinal) {
                return Err(SCHEMA.into());
            }
            previous = Some(section.ordinal);
            section_rows = section_rows
                .checked_add(u64::from(section.records))
                .ok_or(LIMIT)?;
            section_copies = section_copies
                .checked_add(u64::from(section.copies))
                .ok_or(LIMIT)?;
            if section.admitted {
                admitted_rows = admitted_rows
                    .checked_add(u64::from(section.records))
                    .ok_or(LIMIT)?;
                admitted_copies = admitted_copies
                    .checked_add(u64::from(section.copies))
                    .ok_or(LIMIT)?;
            }
            sections.push(section);
        }
        if section_rows != declared_direct_rows
            || section_copies != declared_copy_records
            || admitted_rows != direct_rows
            || admitted_copies != copy_records
        {
            return Err(SCHEMA.into());
        }
        let map = AvailableMapFact {
            file_data_id,
            declared_bytes,
            available_bytes,
            full_ckey_verified,
            encoded_key_authenticated,
            declared_direct_rows,
            declared_copy_records,
            direct_rows,
            copy_records,
            materialized_rows,
            sections,
        };
        if reader.u32()? != 2 {
            return Err(SCHEMA.into());
        }
        let mut liquids = [LiquidFact {
            file_data_id: 0,
            bytes: 0,
            magic: [0; 4],
            direct_rows: 0,
            copy_records: 0,
            materialized_rows: 0,
        }; 2];
        for liquid in &mut liquids {
            *liquid = LiquidFact {
                file_data_id: reader.u32()?,
                bytes: reader.u64()?,
                magic: reader.take(4)?.try_into().map_err(|_| SCHEMA)?,
                direct_rows: reader.u64()?,
                copy_records: reader.u64()?,
                materialized_rows: reader.u64()?,
            };
            if liquid.bytes == 0
                || liquid.direct_rows > 100_000
                || liquid.copy_records > 100_000
                || liquid.materialized_rows < liquid.direct_rows
                || liquid.materialized_rows > liquid.direct_rows.saturating_add(liquid.copy_records)
            {
                return Err(SCHEMA.into());
            }
        }
        if liquids[0].file_data_id != world::MATERIAL_FILE_DATA_ID
            || liquids[1].file_data_id != world::TYPE_FILE_DATA_ID
        {
            return Err(SCHEMA.into());
        }
        let map_count = reader.count(COVERAGE_MAX_MAPS)?;
        if map_count == 0 && matches!(selection, MapSelection::MapAndAncestors(_)) {
            return Err(SCHEMA.into());
        }
        let mut maps = Vec::new();
        maps.try_reserve_exact(map_count)
            .map_err(|_| Error::from("product_allocation"))?;
        let mut previous_id = None;
        for _ in 0..map_count {
            let id = reader.u32()?;
            let has_parent = reader.bit()?;
            let parent = reader.u32()?;
            let entry = DomainMapFact {
                id,
                parent: has_parent.then_some(parent),
                wdt_file_data_id: reader.u32()?,
                depth: reader.u32()?,
                is_parent: reader.bit()?,
                root_absent: matches!(
                    version,
                    ROOT_ABSENT_COVERAGE_VERSION
                        | MODEL_ROOT_ABSENT_COVERAGE_VERSION
                        | REQUEST_CAPACITY_COVERAGE_VERSION
                        | VACATED_WMO_COVERAGE_VERSION
                        | EMPTY_WMO_COVERAGE_VERSION
                        | UNSELECTED_WMO_COVERAGE_VERSION
                        | MATERIAL_FILTERED_WMO_COVERAGE_VERSION
                        | DEPENDENCY_HEAVY_MATERIAL_FILTERED_WMO_COVERAGE_VERSION
                        | SOURCE_HEAVY_MATERIAL_FILTERED_WMO_COVERAGE_VERSION
                        | READ_HEAVY_MATERIAL_FILTERED_WMO_COVERAGE_VERSION
                        | MODEL_HEAVY_MATERIAL_FILTERED_WMO_COVERAGE_VERSION
                        | SOURCE_EXPANDED_MATERIAL_FILTERED_WMO_COVERAGE_VERSION
                        | MODEL_BYTES_EXPANDED_MATERIAL_FILTERED_WMO_COVERAGE_VERSION
                        | SOURCE_EXPANDED_V2_MATERIAL_FILTERED_WMO_COVERAGE_VERSION
                        | PLACEMENT_EXPANDED_MATERIAL_FILTERED_WMO_COVERAGE_VERSION
                        | ASSEMBLY_EXPANDED_MATERIAL_FILTERED_WMO_COVERAGE_VERSION
                        | MODEL_OUTPUT_EXPANDED_MATERIAL_FILTERED_WMO_COVERAGE_VERSION
                        | GAMEOBJECT_EXPANDED_MATERIAL_FILTERED_WMO_COVERAGE_VERSION
                        | GAMEOBJECT_MISSING_ROOT_COVERAGE_VERSION
                        | GAMEOBJECT_ROOT_ENCRYPTED_UNAVAILABLE_COVERAGE_VERSION
                ) && reader.bit()?,
            };
            if (!has_parent && parent != 0)
                || entry.wdt_file_data_id == 0
                || previous_id.is_some_and(|value| value >= id)
            {
                return Err(SCHEMA.into());
            }
            previous_id = Some(id);
            maps.push(entry);
        }
        if let MapSelection::MapAndAncestors(id) = selection
            && !maps.iter().any(|entry| entry.id == id)
        {
            return Err(SCHEMA.into());
        }
        let missing_m2_policy = if matches!(
            version,
            MODEL_ROOT_ABSENT_COVERAGE_VERSION
                | REQUEST_CAPACITY_COVERAGE_VERSION
                | VACATED_WMO_COVERAGE_VERSION
                | EMPTY_WMO_COVERAGE_VERSION
                | UNSELECTED_WMO_COVERAGE_VERSION
                | MATERIAL_FILTERED_WMO_COVERAGE_VERSION
                | DEPENDENCY_HEAVY_MATERIAL_FILTERED_WMO_COVERAGE_VERSION
                | SOURCE_HEAVY_MATERIAL_FILTERED_WMO_COVERAGE_VERSION
                | READ_HEAVY_MATERIAL_FILTERED_WMO_COVERAGE_VERSION
                | MODEL_HEAVY_MATERIAL_FILTERED_WMO_COVERAGE_VERSION
                | SOURCE_EXPANDED_MATERIAL_FILTERED_WMO_COVERAGE_VERSION
                | MODEL_BYTES_EXPANDED_MATERIAL_FILTERED_WMO_COVERAGE_VERSION
                | SOURCE_EXPANDED_V2_MATERIAL_FILTERED_WMO_COVERAGE_VERSION
                | PLACEMENT_EXPANDED_MATERIAL_FILTERED_WMO_COVERAGE_VERSION
                | ASSEMBLY_EXPANDED_MATERIAL_FILTERED_WMO_COVERAGE_VERSION
                | MODEL_OUTPUT_EXPANDED_MATERIAL_FILTERED_WMO_COVERAGE_VERSION
                | GAMEOBJECT_EXPANDED_MATERIAL_FILTERED_WMO_COVERAGE_VERSION
                | GAMEOBJECT_MISSING_ROOT_COVERAGE_VERSION
                | GAMEOBJECT_ROOT_ENCRYPTED_UNAVAILABLE_COVERAGE_VERSION
        ) {
            if !reader.bit()? {
                return Err(SCHEMA.into());
            }
            run::MissingModelPolicy::DeclareRootAbsentWmoDoodadM2
        } else {
            run::MissingModelPolicy::Reject
        };
        let mut missing_m2 = Vec::new();
        if matches!(
            version,
            MODEL_ROOT_ABSENT_COVERAGE_VERSION
                | REQUEST_CAPACITY_COVERAGE_VERSION
                | VACATED_WMO_COVERAGE_VERSION
                | EMPTY_WMO_COVERAGE_VERSION
                | UNSELECTED_WMO_COVERAGE_VERSION
                | MATERIAL_FILTERED_WMO_COVERAGE_VERSION
                | DEPENDENCY_HEAVY_MATERIAL_FILTERED_WMO_COVERAGE_VERSION
                | SOURCE_HEAVY_MATERIAL_FILTERED_WMO_COVERAGE_VERSION
                | READ_HEAVY_MATERIAL_FILTERED_WMO_COVERAGE_VERSION
                | MODEL_HEAVY_MATERIAL_FILTERED_WMO_COVERAGE_VERSION
                | SOURCE_EXPANDED_MATERIAL_FILTERED_WMO_COVERAGE_VERSION
                | MODEL_BYTES_EXPANDED_MATERIAL_FILTERED_WMO_COVERAGE_VERSION
                | SOURCE_EXPANDED_V2_MATERIAL_FILTERED_WMO_COVERAGE_VERSION
                | PLACEMENT_EXPANDED_MATERIAL_FILTERED_WMO_COVERAGE_VERSION
                | ASSEMBLY_EXPANDED_MATERIAL_FILTERED_WMO_COVERAGE_VERSION
                | MODEL_OUTPUT_EXPANDED_MATERIAL_FILTERED_WMO_COVERAGE_VERSION
                | GAMEOBJECT_EXPANDED_MATERIAL_FILTERED_WMO_COVERAGE_VERSION
                | GAMEOBJECT_MISSING_ROOT_COVERAGE_VERSION
                | GAMEOBJECT_ROOT_ENCRYPTED_UNAVAILABLE_COVERAGE_VERSION
        ) {
            let count = reader.count(COVERAGE_MAX_MISSING_M2)?;
            missing_m2
                .try_reserve_exact(count)
                .map_err(|_| Error::from("product_allocation"))?;
            for _ in 0..count {
                missing_m2.push(reader.u32()?);
            }
        }
        validate_missing_m2(&missing_m2, missing_m2_policy)?;
        let resource_profile = if matches!(
            version,
            REQUEST_CAPACITY_COVERAGE_VERSION
                | VACATED_WMO_COVERAGE_VERSION
                | EMPTY_WMO_COVERAGE_VERSION
                | UNSELECTED_WMO_COVERAGE_VERSION
                | MATERIAL_FILTERED_WMO_COVERAGE_VERSION
                | DEPENDENCY_HEAVY_MATERIAL_FILTERED_WMO_COVERAGE_VERSION
                | SOURCE_HEAVY_MATERIAL_FILTERED_WMO_COVERAGE_VERSION
                | READ_HEAVY_MATERIAL_FILTERED_WMO_COVERAGE_VERSION
                | MODEL_HEAVY_MATERIAL_FILTERED_WMO_COVERAGE_VERSION
                | SOURCE_EXPANDED_MATERIAL_FILTERED_WMO_COVERAGE_VERSION
                | MODEL_BYTES_EXPANDED_MATERIAL_FILTERED_WMO_COVERAGE_VERSION
                | SOURCE_EXPANDED_V2_MATERIAL_FILTERED_WMO_COVERAGE_VERSION
                | PLACEMENT_EXPANDED_MATERIAL_FILTERED_WMO_COVERAGE_VERSION
                | ASSEMBLY_EXPANDED_MATERIAL_FILTERED_WMO_COVERAGE_VERSION
                | MODEL_OUTPUT_EXPANDED_MATERIAL_FILTERED_WMO_COVERAGE_VERSION
                | GAMEOBJECT_EXPANDED_MATERIAL_FILTERED_WMO_COVERAGE_VERSION
                | GAMEOBJECT_MISSING_ROOT_COVERAGE_VERSION
                | GAMEOBJECT_ROOT_ENCRYPTED_UNAVAILABLE_COVERAGE_VERSION
        ) {
            let expected_profile = match version {
                DEPENDENCY_HEAVY_MATERIAL_FILTERED_WMO_COVERAGE_VERSION => {
                    resources::Profile::DependencyHeavyV1
                }
                SOURCE_HEAVY_MATERIAL_FILTERED_WMO_COVERAGE_VERSION => {
                    resources::Profile::SourceHeavyV1
                }
                READ_HEAVY_MATERIAL_FILTERED_WMO_COVERAGE_VERSION => {
                    resources::Profile::ReadHeavyV1
                }
                MODEL_HEAVY_MATERIAL_FILTERED_WMO_COVERAGE_VERSION => {
                    resources::Profile::ModelHeavyV1
                }
                SOURCE_EXPANDED_MATERIAL_FILTERED_WMO_COVERAGE_VERSION => {
                    resources::Profile::SourceExpandedV1
                }
                SOURCE_EXPANDED_V2_MATERIAL_FILTERED_WMO_COVERAGE_VERSION => {
                    resources::Profile::SourceExpandedV2
                }
                PLACEMENT_EXPANDED_MATERIAL_FILTERED_WMO_COVERAGE_VERSION => {
                    resources::Profile::PlacementExpandedV1
                }
                ASSEMBLY_EXPANDED_MATERIAL_FILTERED_WMO_COVERAGE_VERSION => {
                    resources::Profile::AssemblyExpandedV1
                }
                MODEL_OUTPUT_EXPANDED_MATERIAL_FILTERED_WMO_COVERAGE_VERSION => {
                    resources::Profile::ModelOutputExpandedV1
                }
                MODEL_BYTES_EXPANDED_MATERIAL_FILTERED_WMO_COVERAGE_VERSION => {
                    resources::Profile::ModelBytesExpandedV1
                }
                GAMEOBJECT_EXPANDED_MATERIAL_FILTERED_WMO_COVERAGE_VERSION => {
                    resources::Profile::GameObjectExpandedV1
                }
                GAMEOBJECT_MISSING_ROOT_COVERAGE_VERSION => {
                    resources::Profile::GameObjectMissingRootExpandedV1
                }
                GAMEOBJECT_ROOT_ENCRYPTED_UNAVAILABLE_COVERAGE_VERSION => {
                    resources::Profile::GameObjectRootAndEncryptedUnavailableExpandedV1
                }
                _ => resources::Profile::RequestHeavyV1,
            };
            if reader.text()? != expected_profile.id() {
                return Err(SCHEMA.into());
            }
            expected_profile
        } else {
            resources::Profile::BoundedV1
        };
        let (missing_wmo_policy, missing_wmo, empty_wmo) = if version
            == VACATED_WMO_COVERAGE_VERSION
        {
            if !reader.bit()? {
                return Err(SCHEMA.into());
            }
            let missing_wmo_policy = run::MissingWmoPolicy::DeclareVacatedAfterRootAbsentM2;
            let count = reader.count(COVERAGE_MAX_MISSING_WMO)?;
            let mut missing_wmo = Vec::new();
            missing_wmo
                .try_reserve_exact(count)
                .map_err(|_| Error::from("product_allocation"))?;
            for _ in 0..count {
                missing_wmo.push(reader.u32()?);
            }
            validate_missing_wmo(&missing_wmo, missing_wmo_policy, &missing_m2)?;
            (missing_wmo_policy, missing_wmo, Vec::new())
        } else if version == EMPTY_WMO_COVERAGE_VERSION {
            if !reader.bit()? {
                return Err(SCHEMA.into());
            }
            let missing_wmo_policy = run::MissingWmoPolicy::DeclareVacatedAfterRootAbsentM2OrEmpty;
            let missing_wmo_count = reader.count(COVERAGE_MAX_MISSING_WMO)?;
            let mut missing_wmo = Vec::new();
            missing_wmo
                .try_reserve_exact(missing_wmo_count)
                .map_err(|_| Error::from("product_allocation"))?;
            for _ in 0..missing_wmo_count {
                missing_wmo.push(reader.u32()?);
            }
            validate_missing_wmo(&missing_wmo, missing_wmo_policy, &missing_m2)?;
            let empty_wmo_count = reader.count(COVERAGE_MAX_EMPTY_WMO)?;
            let mut empty_wmo = Vec::new();
            empty_wmo
                .try_reserve_exact(empty_wmo_count)
                .map_err(|_| Error::from("product_allocation"))?;
            for _ in 0..empty_wmo_count {
                empty_wmo.push(reader.u32()?);
            }
            validate_empty_wmo(&empty_wmo, missing_wmo_policy, &missing_wmo, &missing_m2)?;
            (missing_wmo_policy, missing_wmo, empty_wmo)
        } else if matches!(
            version,
            UNSELECTED_WMO_COVERAGE_VERSION
                | MATERIAL_FILTERED_WMO_COVERAGE_VERSION
                | DEPENDENCY_HEAVY_MATERIAL_FILTERED_WMO_COVERAGE_VERSION
                | SOURCE_HEAVY_MATERIAL_FILTERED_WMO_COVERAGE_VERSION
                | READ_HEAVY_MATERIAL_FILTERED_WMO_COVERAGE_VERSION
                | MODEL_HEAVY_MATERIAL_FILTERED_WMO_COVERAGE_VERSION
                | SOURCE_EXPANDED_MATERIAL_FILTERED_WMO_COVERAGE_VERSION
                | MODEL_BYTES_EXPANDED_MATERIAL_FILTERED_WMO_COVERAGE_VERSION
                | SOURCE_EXPANDED_V2_MATERIAL_FILTERED_WMO_COVERAGE_VERSION
                | PLACEMENT_EXPANDED_MATERIAL_FILTERED_WMO_COVERAGE_VERSION
                | ASSEMBLY_EXPANDED_MATERIAL_FILTERED_WMO_COVERAGE_VERSION
                | MODEL_OUTPUT_EXPANDED_MATERIAL_FILTERED_WMO_COVERAGE_VERSION
                | GAMEOBJECT_EXPANDED_MATERIAL_FILTERED_WMO_COVERAGE_VERSION
                | GAMEOBJECT_MISSING_ROOT_COVERAGE_VERSION
                | GAMEOBJECT_ROOT_ENCRYPTED_UNAVAILABLE_COVERAGE_VERSION
        ) {
            if !reader.bit()? {
                return Err(SCHEMA.into());
            }
            let missing_wmo_policy = if matches!(
                version,
                MATERIAL_FILTERED_WMO_COVERAGE_VERSION
                    | DEPENDENCY_HEAVY_MATERIAL_FILTERED_WMO_COVERAGE_VERSION
                    | SOURCE_HEAVY_MATERIAL_FILTERED_WMO_COVERAGE_VERSION
                    | READ_HEAVY_MATERIAL_FILTERED_WMO_COVERAGE_VERSION
                    | MODEL_HEAVY_MATERIAL_FILTERED_WMO_COVERAGE_VERSION
                    | SOURCE_EXPANDED_MATERIAL_FILTERED_WMO_COVERAGE_VERSION
                    | MODEL_BYTES_EXPANDED_MATERIAL_FILTERED_WMO_COVERAGE_VERSION
                    | SOURCE_EXPANDED_V2_MATERIAL_FILTERED_WMO_COVERAGE_VERSION
                    | PLACEMENT_EXPANDED_MATERIAL_FILTERED_WMO_COVERAGE_VERSION
                    | ASSEMBLY_EXPANDED_MATERIAL_FILTERED_WMO_COVERAGE_VERSION
                    | MODEL_OUTPUT_EXPANDED_MATERIAL_FILTERED_WMO_COVERAGE_VERSION
                    | GAMEOBJECT_EXPANDED_MATERIAL_FILTERED_WMO_COVERAGE_VERSION
                    | GAMEOBJECT_MISSING_ROOT_COVERAGE_VERSION
                    | GAMEOBJECT_ROOT_ENCRYPTED_UNAVAILABLE_COVERAGE_VERSION
            ) {
                run::MissingWmoPolicy::DeclareVacatedAfterRootAbsentM2OrEmptyOrUnselectedOrMaterialFiltered
            } else {
                run::MissingWmoPolicy::DeclareVacatedAfterRootAbsentM2OrEmptyOrUnselected
            };
            let missing_wmo_count = reader.count(COVERAGE_MAX_MISSING_WMO)?;
            let mut missing_wmo = Vec::new();
            missing_wmo
                .try_reserve_exact(missing_wmo_count)
                .map_err(|_| Error::from("product_allocation"))?;
            for _ in 0..missing_wmo_count {
                missing_wmo.push(reader.u32()?);
            }
            validate_missing_wmo(&missing_wmo, missing_wmo_policy, &missing_m2)?;
            let empty_wmo_count = reader.count(COVERAGE_MAX_EMPTY_WMO)?;
            let mut empty_wmo = Vec::new();
            empty_wmo
                .try_reserve_exact(empty_wmo_count)
                .map_err(|_| Error::from("product_allocation"))?;
            for _ in 0..empty_wmo_count {
                empty_wmo.push(reader.u32()?);
            }
            validate_empty_wmo(&empty_wmo, missing_wmo_policy, &missing_wmo, &missing_m2)?;
            (missing_wmo_policy, missing_wmo, empty_wmo)
        } else {
            (run::MissingWmoPolicy::Reject, Vec::new(), Vec::new())
        };
        let mut unselected_wmo = Vec::new();
        if matches!(
            version,
            UNSELECTED_WMO_COVERAGE_VERSION
                | MATERIAL_FILTERED_WMO_COVERAGE_VERSION
                | DEPENDENCY_HEAVY_MATERIAL_FILTERED_WMO_COVERAGE_VERSION
                | SOURCE_HEAVY_MATERIAL_FILTERED_WMO_COVERAGE_VERSION
                | READ_HEAVY_MATERIAL_FILTERED_WMO_COVERAGE_VERSION
                | MODEL_HEAVY_MATERIAL_FILTERED_WMO_COVERAGE_VERSION
                | SOURCE_EXPANDED_MATERIAL_FILTERED_WMO_COVERAGE_VERSION
                | MODEL_BYTES_EXPANDED_MATERIAL_FILTERED_WMO_COVERAGE_VERSION
                | SOURCE_EXPANDED_V2_MATERIAL_FILTERED_WMO_COVERAGE_VERSION
                | PLACEMENT_EXPANDED_MATERIAL_FILTERED_WMO_COVERAGE_VERSION
                | ASSEMBLY_EXPANDED_MATERIAL_FILTERED_WMO_COVERAGE_VERSION
                | MODEL_OUTPUT_EXPANDED_MATERIAL_FILTERED_WMO_COVERAGE_VERSION
                | GAMEOBJECT_EXPANDED_MATERIAL_FILTERED_WMO_COVERAGE_VERSION
                | GAMEOBJECT_MISSING_ROOT_COVERAGE_VERSION
                | GAMEOBJECT_ROOT_ENCRYPTED_UNAVAILABLE_COVERAGE_VERSION
        ) {
            let count = reader.count(COVERAGE_MAX_UNSELECTED_WMO)?;
            unselected_wmo
                .try_reserve_exact(count)
                .map_err(|_| Error::from("product_allocation"))?;
            for _ in 0..count {
                unselected_wmo.push(reader.u32()?);
            }
            validate_unselected_wmo(
                &unselected_wmo,
                missing_wmo_policy,
                &missing_m2,
                &missing_wmo,
                &empty_wmo,
            )?;
        }
        let mut material_filtered_wmo = Vec::new();
        if matches!(
            version,
            MATERIAL_FILTERED_WMO_COVERAGE_VERSION
                | DEPENDENCY_HEAVY_MATERIAL_FILTERED_WMO_COVERAGE_VERSION
                | SOURCE_HEAVY_MATERIAL_FILTERED_WMO_COVERAGE_VERSION
                | READ_HEAVY_MATERIAL_FILTERED_WMO_COVERAGE_VERSION
                | MODEL_HEAVY_MATERIAL_FILTERED_WMO_COVERAGE_VERSION
                | SOURCE_EXPANDED_MATERIAL_FILTERED_WMO_COVERAGE_VERSION
                | MODEL_BYTES_EXPANDED_MATERIAL_FILTERED_WMO_COVERAGE_VERSION
                | SOURCE_EXPANDED_V2_MATERIAL_FILTERED_WMO_COVERAGE_VERSION
                | PLACEMENT_EXPANDED_MATERIAL_FILTERED_WMO_COVERAGE_VERSION
                | ASSEMBLY_EXPANDED_MATERIAL_FILTERED_WMO_COVERAGE_VERSION
                | MODEL_OUTPUT_EXPANDED_MATERIAL_FILTERED_WMO_COVERAGE_VERSION
                | GAMEOBJECT_EXPANDED_MATERIAL_FILTERED_WMO_COVERAGE_VERSION
                | GAMEOBJECT_MISSING_ROOT_COVERAGE_VERSION
                | GAMEOBJECT_ROOT_ENCRYPTED_UNAVAILABLE_COVERAGE_VERSION
        ) {
            let count = reader.count(COVERAGE_MAX_MATERIAL_FILTERED_WMO)?;
            material_filtered_wmo
                .try_reserve_exact(count)
                .map_err(|_| Error::from("product_allocation"))?;
            for _ in 0..count {
                material_filtered_wmo.push(reader.u32()?);
            }
            validate_material_filtered_wmo(
                &material_filtered_wmo,
                missing_wmo_policy,
                &missing_m2,
                &missing_wmo,
                &empty_wmo,
                &unselected_wmo,
            )?;
        }
        let gameobjects = if matches!(
            version,
            GAMEOBJECT_EXPANDED_MATERIAL_FILTERED_WMO_COVERAGE_VERSION
                | GAMEOBJECT_MISSING_ROOT_COVERAGE_VERSION
                | GAMEOBJECT_ROOT_ENCRYPTED_UNAVAILABLE_COVERAGE_VERSION
        ) {
            Some(parse_gameobject_facts(
                &mut reader,
                gameobject_startup_reads,
                version != GAMEOBJECT_EXPANDED_MATERIAL_FILTERED_WMO_COVERAGE_VERSION,
                version == GAMEOBJECT_ROOT_ENCRYPTED_UNAVAILABLE_COVERAGE_VERSION,
            )?)
        } else {
            None
        };
        let gameobject_missing_root = gameobjects
            .as_ref()
            .map_or_else(Vec::new, |facts| facts.missing_root_fdids.clone());
        let gameobject_encrypted_unavailable = gameobjects
            .as_ref()
            .map_or_else(Vec::new, |facts| facts.encrypted_unavailable_fdids.clone());
        let artifact_count = reader.count(COVERAGE_MAX_ARTIFACTS)?;
        let mut artifacts = Vec::new();
        artifacts
            .try_reserve_exact(artifact_count)
            .map_err(|_| Error::from("product_allocation"))?;
        let mut previous_path: Option<String> = None;
        for _ in 0..artifact_count {
            let path = reader.text()?.to_owned();
            let artifact = ArtifactFact {
                path,
                bytes: reader.u64()?,
                sha: reader.take(32)?.try_into().map_err(|_| SCHEMA)?,
            };
            if artifact.path.is_empty()
                || artifact.path.len() > 512
                || !artifact.path.starts_with("vmaps/")
                || previous_path
                    .as_ref()
                    .is_some_and(|previous| previous >= &artifact.path)
            {
                return Err(SCHEMA.into());
            }
            previous_path = Some(artifact.path.clone());
            artifacts.push(artifact);
        }
        if reader.at != bytes.len() {
            return Err(SCHEMA.into());
        }
        let coverage = Self {
            context,
            selection,
            missing_wdt_policy,
            missing_m2_policy,
            missing_wmo_policy,
            resource_profile,
            missing_m2,
            missing_wmo,
            empty_wmo,
            unselected_wmo,
            material_filtered_wmo,
            gameobject_missing_root,
            gameobject_encrypted_unavailable,
            gameobject_scope,
            gameobject_file_data_id,
            gameobject_block_reason,
            gameobject_request_made,
            gameobject_startup_reads,
            gameobjects,
            model_scope,
            liquid_object_scope,
            liquid_object_request_made,
            map,
            liquids,
            maps,
            artifacts,
        };
        if coverage.encode()?.as_slice() != bytes {
            return Err(SCHEMA.into());
        }
        Ok(coverage)
    }
}

fn available_map_fact(
    facts: &forever_casc::collision::available_70374::TableFacts,
) -> Result<AvailableMapFact, Error> {
    let mut sections = Vec::new();
    sections
        .try_reserve_exact(facts.sections().count())
        .map_err(|_| Error::from("product_allocation"))?;
    for section in facts.sections() {
        sections.push(SectionFact {
            ordinal: section.ordinal,
            records: section.declared_records,
            copies: section.declared_copies,
            admitted: section.state == forever_db2::availability::SectionState::Admitted,
        });
    }
    Ok(AvailableMapFact {
        file_data_id: facts.file_data_id(),
        declared_bytes: facts.declared_bytes,
        available_bytes: facts.available_bytes,
        full_ckey_verified: facts.full_ckey_verified,
        encoded_key_authenticated: facts.encoded_key_authenticated,
        declared_direct_rows: facts.declared_direct_rows as u64,
        declared_copy_records: facts.declared_copy_records as u64,
        direct_rows: facts.direct_rows as u64,
        copy_records: facts.copy_records as u64,
        materialized_rows: facts.materialized_rows as u64,
        sections,
    })
}

fn liquid_fact(facts: &TableFacts) -> Result<LiquidFact, Error> {
    Ok(LiquidFact {
        file_data_id: facts.source().file_data_id(),
        bytes: facts.source().byte_count() as u64,
        magic: facts.source().first_magic().ok_or(SCHEMA)?,
        direct_rows: facts.direct_rows as u64,
        copy_records: facts.copy_records as u64,
        materialized_rows: facts.materialized_rows as u64,
    })
}

fn parse_gameobject_facts(
    reader: &mut Reader<'_>,
    startup_reads: u64,
    allow_missing_root: bool,
    allow_encrypted_unavailable: bool,
) -> Result<GameObjectFacts, Error> {
    if reader.u32()? != forever_casc::collision::gameobjects::TABLE_FILE_DATA_ID {
        return Err(SCHEMA.into());
    }
    let declared_bytes = reader.u64()?;
    let available_bytes = reader.u64()?;
    let full_ckey_verified = reader.bit()?;
    let encoded_key_authenticated = reader.bit()?;
    let mut counts = [0_u64; 19];
    for value in &mut counts {
        *value = reader.u64()?;
    }
    let missing_root_rows = if allow_missing_root { reader.u64()? } else { 0 };
    let mut missing_root_fdids = Vec::new();
    if allow_missing_root {
        let count = reader.count(GAMEOBJECT_MISSING_ROOT_MAX)?;
        missing_root_fdids
            .try_reserve_exact(count)
            .map_err(|_| Error::from("product_allocation"))?;
        for _ in 0..count {
            missing_root_fdids.push(reader.u32()?);
        }
    }
    let encrypted_unavailable_rows = if allow_encrypted_unavailable {
        reader.u64()?
    } else {
        0
    };
    let mut encrypted_unavailable_fdids = Vec::new();
    if allow_encrypted_unavailable {
        let count = reader.count(GAMEOBJECT_ENCRYPTED_UNAVAILABLE_MAX)?;
        encrypted_unavailable_fdids
            .try_reserve_exact(count)
            .map_err(|_| Error::from("product_allocation"))?;
        for _ in 0..count {
            encrypted_unavailable_fdids.push(reader.u32()?);
        }
    }
    let temp_sha = reader.take(32)?.try_into().map_err(|_| SCHEMA)?;
    let dtree_sha = reader.take(32)?.try_into().map_err(|_| SCHEMA)?;
    let work_strict_candidates = reader.u64()?;
    let work_partial_candidates = reader.u64()?;
    let work_partial_bytes = reader.u64()?;
    let section_count = reader.count(16)?;
    let mut sections = Vec::new();
    sections
        .try_reserve_exact(section_count)
        .map_err(|_| Error::from("product_allocation"))?;
    for _ in 0..section_count {
        sections.push(GameObjectSectionFact {
            ordinal: reader.u32()?,
            records: reader.u32()?,
            copies: reader.u32()?,
            admitted: reader.bit()?,
        });
    }
    let outcome_count = reader.count(forever_casc::collision::gameobjects::MAX_DIRECT_ROWS)?;
    let mut outcomes = Vec::new();
    outcomes
        .try_reserve_exact(outcome_count)
        .map_err(|_| Error::from("product_allocation"))?;
    for _ in 0..outcome_count {
        let ordinal = reader.u64()?;
        let display_id = reader.u32()?;
        let file_data_id = reader.u32()?;
        let disposition = match reader.u32()? {
            0 => GameObjectDisposition::ZeroFileDataId,
            1 => GameObjectDisposition::Texture,
            2 => GameObjectDisposition::Collision,
            3 => GameObjectDisposition::M2NoCollision,
            4 => GameObjectDisposition::WmoDoodadOnly,
            5 if allow_missing_root => GameObjectDisposition::MissingRoot,
            6 if allow_encrypted_unavailable => GameObjectDisposition::EncryptedUnavailable,
            _ => return Err(SCHEMA.into()),
        };
        outcomes.push(GameObjectOutcomeFact {
            ordinal,
            display_id,
            file_data_id,
            disposition,
        });
    }
    let facts = GameObjectFacts {
        declared_bytes,
        available_bytes,
        full_ckey_verified,
        encoded_key_authenticated,
        declared_direct_rows: counts[0],
        declared_copy_records: counts[1],
        direct_rows: counts[2],
        copy_records: counts[3],
        materialized_rows: counts[4],
        candidate_requests: counts[5],
        direct_outcomes: counts[6],
        zero_fdids: counts[7],
        textures: counts[8],
        m2_no_collision: counts[9],
        wmo_doodad_only: counts[10],
        missing_root_rows,
        missing_root_fdids,
        encrypted_unavailable_rows,
        encrypted_unavailable_fdids,
        collision_rows: counts[11],
        list_rows: counts[12],
        list_m2_rows: counts[13],
        list_wmo_rows: counts[14],
        list_vertex_visits: counts[15],
        list_raw_visit_bytes: counts[16],
        temp_bytes: counts[17],
        dtree_bytes: counts[18],
        temp_sha,
        dtree_sha,
        work_strict_candidates,
        work_partial_candidates,
        work_partial_bytes,
        sections,
        outcomes,
    };
    if startup_reads == 0 {
        return Err(SCHEMA.into());
    }
    validate_gameobject_facts(&facts, allow_missing_root, allow_encrypted_unavailable)?;
    Ok(facts)
}

fn validate_gameobject_facts(
    facts: &GameObjectFacts,
    allow_missing_root: bool,
    allow_encrypted_unavailable: bool,
) -> Result<(), Error> {
    let max_work_candidates =
        u64::try_from(forever_casc::local_store::Limits::default().max_candidate_attempts)
            .map_err(|_| LIMIT)?;
    let work_candidates = facts
        .work_strict_candidates
        .checked_add(facts.work_partial_candidates);
    let disposition_total = facts
        .zero_fdids
        .checked_add(facts.textures)
        .and_then(|value| value.checked_add(facts.m2_no_collision))
        .and_then(|value| value.checked_add(facts.wmo_doodad_only))
        .and_then(|value| value.checked_add(facts.missing_root_rows))
        .and_then(|value| value.checked_add(facts.encrypted_unavailable_rows))
        .and_then(|value| value.checked_add(facts.collision_rows));
    let list_rows = facts.list_m2_rows.checked_add(facts.list_wmo_rows);
    if facts.sections.len() > 16
        || facts.outcomes.len() as u64 != facts.direct_outcomes
        || facts.direct_outcomes != facts.direct_rows
        || facts.materialized_rows != facts.direct_rows
        || facts.copy_records > facts.declared_copy_records
        || facts.direct_rows > facts.declared_direct_rows
        || work_candidates.is_none_or(|value| value > max_work_candidates)
        || facts.available_bytes > facts.declared_bytes
        || (facts.full_ckey_verified && facts.available_bytes != facts.declared_bytes)
        || (!facts.full_ckey_verified && !facts.encoded_key_authenticated)
        || disposition_total != Some(facts.direct_outcomes)
        || (!allow_missing_root
            && (facts.missing_root_rows != 0 || !facts.missing_root_fdids.is_empty()))
        || (allow_missing_root
            && (facts.missing_root_fdids.len() > GAMEOBJECT_MISSING_ROOT_MAX
                || facts.missing_root_fdids.contains(&0)
                || facts
                    .missing_root_fdids
                    .windows(2)
                    .any(|pair| pair[0] >= pair[1])))
        || (!allow_encrypted_unavailable
            && (facts.encrypted_unavailable_rows != 0
                || !facts.encrypted_unavailable_fdids.is_empty()))
        || (allow_encrypted_unavailable
            && (facts.encrypted_unavailable_fdids.len() > GAMEOBJECT_ENCRYPTED_UNAVAILABLE_MAX
                || facts.encrypted_unavailable_fdids.contains(&0)
                || facts
                    .encrypted_unavailable_fdids
                    .windows(2)
                    .any(|pair| pair[0] >= pair[1])))
        || facts.list_rows != facts.collision_rows
        || list_rows != Some(facts.list_rows)
        || facts.list_rows > forever_vmap::gameobject_models::MAX_ROWS as u64
        || facts.temp_bytes > forever_vmap::gameobject_models::MAX_LIST_BYTES as u64
        || facts.dtree_bytes > forever_vmap::gameobject_models::MAX_LIST_BYTES as u64
        || facts.temp_bytes < forever_vmap::gameobject_models::TEMP_MAGIC.len() as u64
        || facts.dtree_bytes < forever_vmap::gameobject_models::DTREE_MAGIC.len() as u64
    {
        return Err(SCHEMA.into());
    }
    let mut declared_rows = 0_u64;
    let mut declared_copies = 0_u64;
    let mut admitted_rows = 0_u64;
    let mut admitted_copies = 0_u64;
    let mut previous_section = None;
    for section in &facts.sections {
        if section.ordinal >= 16
            || previous_section.is_some_and(|previous| previous >= section.ordinal)
        {
            return Err(SCHEMA.into());
        }
        previous_section = Some(section.ordinal);
        declared_rows = declared_rows
            .checked_add(u64::from(section.records))
            .ok_or(LIMIT)?;
        declared_copies = declared_copies
            .checked_add(u64::from(section.copies))
            .ok_or(LIMIT)?;
        if section.admitted {
            admitted_rows = admitted_rows
                .checked_add(u64::from(section.records))
                .ok_or(LIMIT)?;
            admitted_copies = admitted_copies
                .checked_add(u64::from(section.copies))
                .ok_or(LIMIT)?;
        }
    }
    if declared_rows != facts.declared_direct_rows
        || declared_copies != facts.declared_copy_records
        || admitted_rows != facts.direct_rows
        || admitted_copies != facts.copy_records
    {
        return Err(SCHEMA.into());
    }
    let mut previous_ordinal = None;
    let mut outcomes = [0_u64; 7];
    let mut encrypted_candidate_rows = 0_u64;
    for (index, outcome) in facts.outcomes.iter().enumerate() {
        if outcome.ordinal != index as u64
            || previous_ordinal.is_some_and(|previous| previous >= outcome.ordinal)
            || (outcome.disposition == GameObjectDisposition::ZeroFileDataId)
                != (outcome.file_data_id == 0)
            || (outcome.disposition == GameObjectDisposition::MissingRoot && !allow_missing_root)
            || (outcome.disposition == GameObjectDisposition::EncryptedUnavailable
                && !allow_encrypted_unavailable)
        {
            return Err(SCHEMA.into());
        }
        previous_ordinal = Some(outcome.ordinal);
        if outcome.disposition == GameObjectDisposition::EncryptedUnavailable {
            encrypted_candidate_rows = encrypted_candidate_rows.checked_add(1).ok_or(LIMIT)?;
        }
        let bucket = match outcome.disposition {
            GameObjectDisposition::ZeroFileDataId => 0,
            GameObjectDisposition::Texture => 1,
            GameObjectDisposition::Collision => 2,
            GameObjectDisposition::M2NoCollision => 3,
            GameObjectDisposition::WmoDoodadOnly => 4,
            GameObjectDisposition::MissingRoot => 5,
            GameObjectDisposition::EncryptedUnavailable => 6,
        };
        outcomes[bucket] = outcomes[bucket].checked_add(1).ok_or(LIMIT)?;
    }
    if outcomes
        != [
            facts.zero_fdids,
            facts.textures,
            facts.collision_rows,
            facts.m2_no_collision,
            facts.wmo_doodad_only,
            facts.missing_root_rows,
            facts.encrypted_unavailable_rows,
        ]
        || encrypted_candidate_rows != facts.encrypted_unavailable_rows
    {
        return Err(SCHEMA.into());
    }
    let mut missing_root_fdids = Vec::new();
    missing_root_fdids
        .try_reserve_exact(usize::try_from(facts.missing_root_rows).map_err(|_| LIMIT)?)
        .map_err(|_| Error::from("product_allocation"))?;
    missing_root_fdids.extend(
        facts
            .outcomes
            .iter()
            .filter(|outcome| outcome.disposition == GameObjectDisposition::MissingRoot)
            .map(|outcome| outcome.file_data_id),
    );
    missing_root_fdids.sort_unstable();
    missing_root_fdids.dedup();
    if missing_root_fdids != facts.missing_root_fdids {
        return Err(SCHEMA.into());
    }
    let mut encrypted_unavailable_fdids = Vec::new();
    encrypted_unavailable_fdids
        .try_reserve_exact(usize::try_from(facts.encrypted_unavailable_rows).map_err(|_| LIMIT)?)
        .map_err(|_| Error::from("product_allocation"))?;
    encrypted_unavailable_fdids.extend(
        facts
            .outcomes
            .iter()
            .filter(|outcome| outcome.disposition == GameObjectDisposition::EncryptedUnavailable)
            .map(|outcome| outcome.file_data_id),
    );
    encrypted_unavailable_fdids.sort_unstable();
    encrypted_unavailable_fdids.dedup();
    if encrypted_unavailable_fdids != facts.encrypted_unavailable_fdids
        || facts
            .missing_root_fdids
            .iter()
            .any(|id| facts.encrypted_unavailable_fdids.binary_search(id).is_ok())
    {
        return Err(SCHEMA.into());
    }
    Ok(())
}

fn encode_gameobject_facts(
    writer: &mut Writer,
    facts: &GameObjectFacts,
    allow_missing_root: bool,
    allow_encrypted_unavailable: bool,
) -> Result<(), Error> {
    validate_gameobject_facts(facts, allow_missing_root, allow_encrypted_unavailable)?;
    writer.u32(forever_casc::collision::gameobjects::TABLE_FILE_DATA_ID)?;
    writer.u64(facts.declared_bytes)?;
    writer.u64(facts.available_bytes)?;
    writer.bit(facts.full_ckey_verified)?;
    writer.bit(facts.encoded_key_authenticated)?;
    for value in [
        facts.declared_direct_rows,
        facts.declared_copy_records,
        facts.direct_rows,
        facts.copy_records,
        facts.materialized_rows,
        facts.candidate_requests,
        facts.direct_outcomes,
        facts.zero_fdids,
        facts.textures,
        facts.m2_no_collision,
        facts.wmo_doodad_only,
        facts.collision_rows,
        facts.list_rows,
        facts.list_m2_rows,
        facts.list_wmo_rows,
        facts.list_vertex_visits,
        facts.list_raw_visit_bytes,
        facts.temp_bytes,
        facts.dtree_bytes,
    ] {
        writer.u64(value)?;
    }
    if allow_missing_root {
        writer.u64(facts.missing_root_rows)?;
        writer.u32(u32::try_from(facts.missing_root_fdids.len()).map_err(|_| LIMIT)?)?;
        for &id in &facts.missing_root_fdids {
            writer.u32(id)?;
        }
    }
    if allow_encrypted_unavailable {
        writer.u64(facts.encrypted_unavailable_rows)?;
        writer.u32(u32::try_from(facts.encrypted_unavailable_fdids.len()).map_err(|_| LIMIT)?)?;
        for &id in &facts.encrypted_unavailable_fdids {
            writer.u32(id)?;
        }
    }
    writer.put(&facts.temp_sha)?;
    writer.put(&facts.dtree_sha)?;
    writer.u64(facts.work_strict_candidates)?;
    writer.u64(facts.work_partial_candidates)?;
    writer.u64(facts.work_partial_bytes)?;
    writer.u32(u32::try_from(facts.sections.len()).map_err(|_| LIMIT)?)?;
    for section in &facts.sections {
        writer.u32(section.ordinal)?;
        writer.u32(section.records)?;
        writer.u32(section.copies)?;
        writer.bit(section.admitted)?;
    }
    writer.u32(u32::try_from(facts.outcomes.len()).map_err(|_| LIMIT)?)?;
    for outcome in &facts.outcomes {
        writer.u64(outcome.ordinal)?;
        writer.u32(outcome.display_id)?;
        writer.u32(outcome.file_data_id)?;
        writer.u32(match outcome.disposition {
            GameObjectDisposition::ZeroFileDataId => 0,
            GameObjectDisposition::Texture => 1,
            GameObjectDisposition::Collision => 2,
            GameObjectDisposition::M2NoCollision => 3,
            GameObjectDisposition::WmoDoodadOnly => 4,
            GameObjectDisposition::MissingRoot if allow_missing_root => 5,
            GameObjectDisposition::MissingRoot => return Err(SCHEMA.into()),
            GameObjectDisposition::EncryptedUnavailable if allow_encrypted_unavailable => 6,
            GameObjectDisposition::EncryptedUnavailable => return Err(SCHEMA.into()),
        })?;
    }
    Ok(())
}

struct Writer {
    bytes: Vec<u8>,
    maximum: usize,
}
impl Writer {
    fn new(maximum: usize) -> Self {
        Self {
            bytes: Vec::new(),
            maximum,
        }
    }
    fn put(&mut self, value: &[u8]) -> Result<(), Error> {
        let required = self.bytes.len().checked_add(value.len()).ok_or(LIMIT)?;
        if required > self.maximum {
            return Err(LIMIT.into());
        }
        if required > self.bytes.capacity() {
            self.bytes
                .try_reserve_exact(required - self.bytes.len())
                .map_err(|_| Error::from("product_allocation"))?;
            if self.bytes.capacity() > self.maximum {
                return Err(LIMIT.into());
            }
        }
        self.bytes.extend_from_slice(value);
        Ok(())
    }
    fn u32(&mut self, value: u32) -> Result<(), Error> {
        self.put(&value.to_le_bytes())
    }
    fn u64(&mut self, value: u64) -> Result<(), Error> {
        self.put(&value.to_le_bytes())
    }
    fn bit(&mut self, value: bool) -> Result<(), Error> {
        self.put(&[u8::from(value)])
    }
    fn text(&mut self, value: &str) -> Result<(), Error> {
        if value.len() > 4096 {
            return Err(LIMIT.into());
        }
        self.u32(u32::try_from(value.len()).map_err(|_| LIMIT)?)?;
        self.put(value.as_bytes())
    }
}

struct Reader<'a> {
    bytes: &'a [u8],
    at: usize,
}
impl<'a> Reader<'a> {
    fn take(&mut self, count: usize) -> Result<&'a [u8], Error> {
        let end = self.at.checked_add(count).ok_or(SCHEMA)?;
        let value = self.bytes.get(self.at..end).ok_or(SCHEMA)?;
        self.at = end;
        Ok(value)
    }
    fn u32(&mut self) -> Result<u32, Error> {
        Ok(u32::from_le_bytes(
            self.take(4)?.try_into().map_err(|_| SCHEMA)?,
        ))
    }
    fn u64(&mut self) -> Result<u64, Error> {
        Ok(u64::from_le_bytes(
            self.take(8)?.try_into().map_err(|_| SCHEMA)?,
        ))
    }
    fn bit(&mut self) -> Result<bool, Error> {
        match self.take(1)?[0] {
            0 => Ok(false),
            1 => Ok(true),
            _ => Err(SCHEMA.into()),
        }
    }
    fn count(&mut self, maximum: usize) -> Result<usize, Error> {
        let count = usize::try_from(self.u32()?).map_err(|_| LIMIT)?;
        if count > maximum {
            return Err(LIMIT.into());
        }
        Ok(count)
    }
    fn text(&mut self) -> Result<&'a str, Error> {
        let length = self.count(4096)?;
        str::from_utf8(self.take(length)?).map_err(|_| SCHEMA.into())
    }
}

#[derive(Clone, Copy)]
struct ModelIdentity {
    class: Class,
    raw_sha: [u8; 32],
}
struct StagedModel {
    identity: ModelIdentity,
    bounds: Option<[u32; 6]>,
    bytes: Vec<u8>,
}
struct Sink<'a> {
    run: &'a mut Run,
    models: BTreeMap<String, StagedModel>,
    placements: BTreeMap<u32, Vec<u8>>,
    model_bytes: usize,
    model_limit: usize,
    max_models: usize,
    model_count_limit: &'static str,
    model_bytes_limit: &'static str,
    placement_bytes: usize,
    placement_limit: usize,
    placement_map_limit: usize,
    placement_capacity: usize,
    placement_error_codes: PlacementErrorCodes,
    failure: Option<Error>,
}
impl Sink<'_> {
    fn reject(&mut self, error: Error) -> Result<(), run::SinkError> {
        if self.failure.is_none() {
            self.failure = Some(error);
        }
        Err(run::SinkError::Rejected)
    }
    fn stage_model(&mut self, model: &forever_casc::collision::Model) -> Result<(), Error> {
        if model.status() != Status::Collision {
            return Ok(());
        }
        let raw_bytes = model.raw().ok_or("product_selected_geometry_model")?;
        let name = str::from_utf8(model.output_basename())
            .map_err(|_| Error::from("product_selected_geometry_model"))?
            .to_owned();
        if name.is_empty()
            || name.len() > placement::MAX_NAME_BYTES
            || name.contains(['/', '\\'])
            || !name.is_ascii()
        {
            return Err("product_selected_geometry_model".into());
        }
        let identity = ModelIdentity {
            class: model.class(),
            raw_sha: integrity::digest(raw_bytes),
        };
        if let Some(previous) = self.models.get(&name) {
            if previous.identity.class == identity.class
                && previous.identity.raw_sha == identity.raw_sha
            {
                return Ok(());
            }
            return Err("product_selected_geometry_model_alias".into());
        }
        admit_model_count(self.models.len(), self.max_models, self.model_count_limit)
            .map_err(Error::from)?;
        let remaining = self
            .model_limit
            .checked_sub(self.model_bytes)
            .ok_or(self.model_bytes_limit)?;
        let limits = vmo::Limits::default();
        let (converted, bounds) = match model.class() {
            Class::M2 => {
                let raw = RawM2::parse(raw_bytes, forever_vmap::Limits::default())
                    .map_err(|error| Error::from(error.code()))?;
                (vmo::convert_m2(&raw, limits), Some(raw.bounds_bits()))
            }
            Class::Wmo => {
                let raw = raw::RawWmo::parse(raw_bytes, raw::ConversionLimits::default())
                    .map_err(|error| Error::from(error.code()))?;
                (vmo::convert_wmo(&raw, limits), None)
            }
        };
        let converted = converted.map_err(|error| Error::from(error.code()))?;
        let bytes = converted.into_bytes();
        let next = self
            .model_bytes
            .checked_add(bytes.len())
            .ok_or(self.model_bytes_limit)?;
        if next > self.model_limit || bytes.len() > remaining {
            return Err(self.model_bytes_limit.into());
        }
        self.models.insert(
            name.to_owned(),
            StagedModel {
                identity,
                bounds,
                bytes,
            },
        );
        self.model_bytes = next;
        Ok(())
    }
    fn stage_placements(&mut self, context: run::Context, bytes: &[u8]) -> Result<(), Error> {
        let target = self.placements.entry(context.map_id).or_default();
        let (next_total, next_len) = next_placement_lengths(
            self.placement_bytes,
            target.len(),
            bytes.len(),
            self.placement_limit,
            self.placement_map_limit,
            self.placement_error_codes,
        )?;
        if next_len > target.capacity() {
            let staged_peak = self
                .placement_capacity
                .checked_add(next_len)
                .ok_or(self.placement_error_codes.aggregate)?;
            if staged_peak > self.placement_limit {
                return Err(self.placement_error_codes.aggregate.into());
            }
            let old_capacity = target.capacity();
            target
                .try_reserve_exact(bytes.len())
                .map_err(|_| Error::from("product_allocation"))?;
            if target.capacity() > placement::MAX_BYTES {
                return Err(self.placement_error_codes.map.into());
            }
            self.placement_capacity = self
                .placement_capacity
                .checked_sub(old_capacity)
                .and_then(|capacity| capacity.checked_add(target.capacity()))
                .ok_or(self.placement_error_codes.aggregate)?;
            if self.placement_capacity > self.placement_limit {
                return Err(self.placement_error_codes.aggregate.into());
            }
        }
        target.extend_from_slice(bytes);
        self.placement_bytes = next_total;
        Ok(())
    }
}

fn next_placement_lengths(
    current_total: usize,
    current_map: usize,
    incoming: usize,
    total_limit: usize,
    map_limit: usize,
    error_codes: PlacementErrorCodes,
) -> Result<(usize, usize), Error> {
    let next_total = current_total
        .checked_add(incoming)
        .ok_or(error_codes.aggregate)?;
    if next_total > total_limit {
        return Err(error_codes.aggregate.into());
    }
    let next_map = current_map.checked_add(incoming).ok_or(error_codes.map)?;
    if next_map > placement::MAX_BYTES || next_map > map_limit {
        return Err(error_codes.map.into());
    }
    Ok((next_total, next_map))
}

impl run::Sink for Sink<'_> {
    fn model(&mut self, model: &forever_casc::collision::Model) -> Result<(), run::SinkError> {
        match self.stage_model(model) {
            Ok(()) => Ok(()),
            Err(error) => self.reject(error),
        }
    }
    fn placements(&mut self, context: run::Context, bytes: &[u8]) -> Result<(), run::SinkError> {
        match self.stage_placements(context, bytes) {
            Ok(()) => Ok(()),
            Err(error) => self.reject(error),
        }
    }
}

fn as_usize(value: u64) -> Result<usize, Error> {
    usize::try_from(value).map_err(|_| Error::from(LIMIT))
}

fn validate_domain_count(
    selection: MapSelection,
    count: usize,
    domain_limit_code: &'static str,
) -> Result<(), Error> {
    if count > COVERAGE_MAX_MAPS
        || (count == 0 && matches!(selection, MapSelection::MapAndAncestors(_)))
    {
        Err(domain_limit_code.into())
    } else {
        Ok(())
    }
}

fn validate_placement_domain(
    placement_map_ids: impl IntoIterator<Item = u32>,
    domain_ids: &[u32],
    error_code: &'static str,
) -> Result<(), Error> {
    for id in placement_map_ids {
        if domain_ids.binary_search(&id).is_err() {
            return Err(error_code.into());
        }
    }
    Ok(())
}

fn geometry_limits(
    budget: DiskBudget,
    resource_profile: resources::Profile,
) -> Result<geometry::Limits, Error> {
    let mut limits = geometry::Limits {
        startup: resource_profile.startup_limits(),
        ..Default::default()
    };
    let raw_budget = as_usize(budget.raw_bytes)?;
    let wdt_budget = as_usize(budget.wdt_bytes)?;
    let cache_budget = as_usize(budget.cache_bytes)?;
    let placement_budget = as_usize(budget.placement_bytes)?;
    limits.startup.collision.max_source_bytes =
        limits.startup.collision.max_source_bytes.min(raw_budget);
    limits.startup.collision.max_raw_bytes = limits.startup.collision.max_raw_bytes.min(raw_budget);
    limits.world.run.max_wdt_memory_bytes = limits.world.run.max_wdt_memory_bytes.min(wdt_budget);
    limits.world.run.max_cache_memory_bytes =
        limits.world.run.max_cache_memory_bytes.min(cache_budget);
    limits.world.run.max_output_bytes = limits.world.run.max_output_bytes.min(
        budget
            .raw_bytes
            .checked_add(budget.placement_bytes)
            .ok_or(LIMIT)?,
    );
    if matches!(
        resource_profile,
        resources::Profile::ModelHeavyV1
            | resources::Profile::SourceExpandedV1
            | resources::Profile::ModelOutputExpandedV1
            | resources::Profile::ModelBytesExpandedV1
            | resources::Profile::SourceExpandedV2
            | resources::Profile::PlacementExpandedV1
            | resources::Profile::AssemblyExpandedV1
            | resources::Profile::GameObjectExpandedV1
            | resources::Profile::GameObjectMissingRootExpandedV1
            | resources::Profile::GameObjectRootAndEncryptedUnavailableExpandedV1
    ) {
        limits.world.run.placements.max_models = resource_profile
            .stage_limits()
            .world
            .world
            .run
            .placements
            .max_models;
    } else {
        limits.world.run.placements.max_models = limits
            .world
            .run
            .placements
            .max_models
            .min(resource_profile.collector_limits().max_models);
    }
    if placement_budget == 0 || raw_budget == 0 {
        return Err(LIMIT.into());
    }
    Ok(limits)
}

fn map_paths(id: u32) -> (String, String) {
    (
        format!("vmaps/{id:04}/{id:04}.vmtree"),
        format!("vmaps/coverage/{id:04}.placements"),
    )
}

fn tile_path(map: u32, x: u32, y: u32, index: bool) -> String {
    format!(
        "vmaps/{map:04}/{map:04}_{x:02}_{y:02}.{}",
        if index { "vmtileidx" } else { "vmtile" }
    )
}

fn publish_gameobject_tree(
    completed: &geometry::Completed,
    sink: &mut Sink<'_>,
    preflight: &Preflight,
    artifacts: &mut BTreeMap<String, ArtifactFact>,
    used_models: &mut BTreeMap<String, ()>,
) -> Result<(), Error> {
    let data = completed.gameobjects();
    let lists = data.lists();
    let facts = completed.gameobject_facts().ok_or(SCHEMA)?;
    let mut actual_outcomes = data.outcome_facts();
    for expected in &facts.outcomes {
        if actual_outcomes.next() != Some(*expected) {
            return Err(SCHEMA.into());
        }
    }
    if actual_outcomes.next().is_some() {
        return Err(SCHEMA.into());
    }
    let tree_bytes = lists.dtree_bytes();
    if facts.dtree_bytes != tree_bytes.len() as u64
        || facts.dtree_sha256 != integrity::digest(tree_bytes)
        || facts.temp_bytes != lists.temp_bytes().len() as u64
        || facts.temp_sha256 != integrity::digest(lists.temp_bytes())
    {
        return Err(SCHEMA.into());
    }
    let tree = forever_vmap::gameobject_models::Dtree::parse(
        tree_bytes,
        forever_vmap::gameobject_models::Limits::default(),
    )
    .map_err(|_| Error::from(SCHEMA))?;
    if tree.row_count() as u64 != facts.collision_rows {
        return Err(SCHEMA.into());
    }
    let temp_size = usize::try_from(facts.temp_bytes).map_err(|_| LIMIT)?;
    let mut reconstructed_temp = Vec::new();
    reconstructed_temp
        .try_reserve_exact(temp_size)
        .map_err(|_| Error::from("product_allocation"))?;
    reconstructed_temp.extend_from_slice(forever_vmap::gameobject_models::TEMP_MAGIC);
    let mut collision_models = data.collision_model_handles();
    for row in tree.rows() {
        let (fact, handle) = collision_models.next().ok_or(SCHEMA)?;
        let model = completed.model(handle).ok_or(SCHEMA)?;
        let name = m2_file_name(fact.file_data_id);
        if fact.outcome != forever_casc::collision::available_70374::GameObjectOutcome::Collision
            || row.display_id() != fact.display_id
            || row.name() != name.as_bytes()
            || model.output_basename() != name.as_bytes()
            || model.status() != Status::Collision
        {
            return Err(SCHEMA.into());
        }
        reconstructed_temp.extend_from_slice(&row.display_id().to_le_bytes());
        reconstructed_temp.extend_from_slice(
            &u32::try_from(row.name().len())
                .map_err(|_| LIMIT)?
                .to_le_bytes(),
        );
        reconstructed_temp.extend_from_slice(row.name());
        register_model(sink, used_models, model)?;
    }
    if collision_models.next().is_some()
        || reconstructed_temp.len() != temp_size
        || reconstructed_temp.as_slice() != lists.temp_bytes()
        || facts.temp_sha256 != integrity::digest(&reconstructed_temp)
    {
        return Err(SCHEMA.into());
    }
    publish(
        sink.run,
        preflight,
        artifacts,
        GAMEOBJECT_MODELS_DTREE_PATH.to_owned(),
        tree_bytes,
    )
}

fn register_model(
    sink: &mut Sink<'_>,
    used_models: &mut BTreeMap<String, ()>,
    model: &forever_casc::collision::Model,
) -> Result<String, Error> {
    if model.status() != Status::Collision {
        return Err("product_selected_geometry_model_binding".into());
    }
    let name = str::from_utf8(model.output_basename())
        .map_err(|_| Error::from("product_selected_geometry_model"))?
        .to_owned();
    sink.stage_model(model)?;
    used_models.insert(name.clone(), ());
    Ok(name)
}

fn publish(
    run: &mut Run,
    preflight: &Preflight,
    facts: &mut BTreeMap<String, ArtifactFact>,
    path: String,
    bytes: &[u8],
) -> Result<(), Error> {
    let sha = integrity::digest(bytes);
    if let Some(previous) = facts.get(&path) {
        if previous.bytes != bytes.len() as u64 || previous.sha != sha {
            return Err("product_selected_geometry_artifact_alias".into());
        }
        return Ok(());
    }
    if facts.len() >= COVERAGE_MAX_ARTIFACTS {
        return Err(LIMIT.into());
    }
    preflight.verify_unchanged()?;
    run.put(Component::Collision, &path, bytes)?;
    preflight.verify_unchanged()?;
    facts.insert(
        path.clone(),
        ArtifactFact {
            path,
            bytes: bytes.len() as u64,
            sha,
        },
    );
    Ok(())
}

pub(crate) fn package_limits_v28_with_extra_files(
    budget: DiskBudget,
    extra_files: usize,
) -> Limits {
    Limits {
        total_bytes: budget
            .output_bytes
            .checked_add(super::DB2_TOTAL_BYTES)
            .expect("v28 package byte budget must fit u64"),
        files: PACKAGE_MAX_FILES
            .checked_add(extra_files)
            .expect("v28 package file budget must fit usize"),
        ..Default::default()
    }
}

pub(crate) fn extract(
    plan: &crate::plan::Plan,
    store: &mut forever_casc::local_store::LocalStore,
    run: &mut Run,
    preflight: &Preflight,
    context: Context,
    budget: DiskBudget,
) -> Result<(), Error> {
    if !plan.is_v22()
        || context.build != 70450
        || context.components != Component::Collision.bit()
        || plan.map_domain.is_none()
    {
        return Err("product_plan_component_scope".into());
    }
    let selection =
        MapSelection::MapAndAncestors(plan.map_domain.ok_or("product_plan_map_domain")?);
    extract_inner(
        store,
        run,
        preflight,
        context,
        budget,
        GeometryPolicy::new(
            selection,
            run::MissingWdtPolicy::Reject,
            run::MissingModelPolicy::Reject,
            resources::Profile::BoundedV1,
        ),
    )
}

pub(crate) fn extract_v28(
    plan: &crate::plan::Plan,
    store: &mut forever_casc::local_store::LocalStore,
    run: &mut Run,
    preflight: &Preflight,
    context: Context,
    budget: DiskBudget,
) -> Result<(), Error> {
    if !plan.is_v28()
        || context.build != 70450
        || context.components != Component::Db2.bit() | Component::Collision.bit()
        || plan.map_domain.is_none()
    {
        return Err("product_plan_component_scope".into());
    }
    let selection =
        MapSelection::MapAndAncestors(plan.map_domain.ok_or("product_plan_map_domain")?);
    extract_inner(
        store,
        run,
        preflight,
        context,
        budget,
        GeometryPolicy::new(
            selection,
            run::MissingWdtPolicy::Reject,
            run::MissingModelPolicy::Reject,
            resources::Profile::BoundedV1,
        ),
    )
}

pub(crate) fn extract_v29(
    plan: &crate::plan::Plan,
    store: &mut forever_casc::local_store::LocalStore,
    run: &mut Run,
    preflight: &Preflight,
    context: Context,
    budget: DiskBudget,
) -> Result<(), Error> {
    if !plan.is_v29()
        || context.build != 70450
        || context.components != Component::Db2.bit() | Component::Collision.bit()
        || plan.map_domain.is_some()
    {
        return Err("product_plan_component_scope".into());
    }
    extract_inner(
        store,
        run,
        preflight,
        context,
        budget,
        GeometryPolicy::new(
            MapSelection::All,
            run::MissingWdtPolicy::Reject,
            run::MissingModelPolicy::Reject,
            resources::Profile::BoundedV1,
        ),
    )
}

pub(crate) fn extract_v30(
    plan: &crate::plan::Plan,
    store: &mut forever_casc::local_store::LocalStore,
    run: &mut Run,
    preflight: &Preflight,
    context: Context,
    budget: DiskBudget,
) -> Result<(), Error> {
    if !plan.is_v30()
        || context.build != 70450
        || context.components != Component::Db2.bit() | Component::Collision.bit()
        || plan.map_domain.is_some()
    {
        return Err("product_plan_component_scope".into());
    }
    extract_inner(
        store,
        run,
        preflight,
        context,
        budget,
        GeometryPolicy::new(
            MapSelection::All,
            run::MissingWdtPolicy::DeclareRootAbsent,
            run::MissingModelPolicy::Reject,
            resources::Profile::BoundedV1,
        ),
    )
}

pub(crate) fn extract_v31(
    plan: &crate::plan::Plan,
    store: &mut forever_casc::local_store::LocalStore,
    run: &mut Run,
    preflight: &Preflight,
    context: Context,
    budget: DiskBudget,
) -> Result<(), Error> {
    if !plan.is_v31()
        || context.build != 70450
        || context.components != Component::Db2.bit() | Component::Collision.bit()
        || plan.map_domain.is_some()
    {
        return Err("product_plan_component_scope".into());
    }
    extract_inner(
        store,
        run,
        preflight,
        context,
        budget,
        GeometryPolicy::new(
            MapSelection::All,
            run::MissingWdtPolicy::DeclareRootAbsent,
            run::MissingModelPolicy::DeclareRootAbsentWmoDoodadM2,
            resources::Profile::BoundedV1,
        ),
    )
}

pub(crate) fn extract_v32(
    plan: &crate::plan::Plan,
    store: &mut forever_casc::local_store::LocalStore,
    run: &mut Run,
    preflight: &Preflight,
    context: Context,
    budget: DiskBudget,
) -> Result<(), Error> {
    if !plan.is_v32()
        || context.build != 70450
        || context.components != Component::Db2.bit() | Component::Collision.bit()
        || plan.map_domain.is_some()
    {
        return Err("product_plan_component_scope".into());
    }
    extract_inner(
        store,
        run,
        preflight,
        context,
        budget,
        GeometryPolicy::new(
            MapSelection::All,
            run::MissingWdtPolicy::DeclareRootAbsent,
            run::MissingModelPolicy::DeclareRootAbsentWmoDoodadM2,
            resources::Profile::RequestHeavyV1,
        ),
    )
}

pub(crate) fn extract_v33(
    plan: &crate::plan::Plan,
    store: &mut forever_casc::local_store::LocalStore,
    run: &mut Run,
    preflight: &Preflight,
    context: Context,
    budget: DiskBudget,
) -> Result<(), Error> {
    if !plan.is_v33()
        || context.build != 70450
        || context.components != Component::Db2.bit() | Component::Collision.bit()
        || plan.map_domain.is_some()
    {
        return Err("product_plan_component_scope".into());
    }
    extract_inner(
        store,
        run,
        preflight,
        context,
        budget,
        GeometryPolicy::new_with_missing_wmo(
            MapSelection::All,
            run::MissingWdtPolicy::DeclareRootAbsent,
            run::MissingModelPolicy::DeclareRootAbsentWmoDoodadM2,
            run::MissingWmoPolicy::DeclareVacatedAfterRootAbsentM2,
            resources::Profile::RequestHeavyV1,
        ),
    )
}

pub(crate) fn extract_v34(
    plan: &crate::plan::Plan,
    store: &mut forever_casc::local_store::LocalStore,
    run: &mut Run,
    preflight: &Preflight,
    context: Context,
    budget: DiskBudget,
) -> Result<(), Error> {
    if !plan.is_v34()
        || context.build != 70450
        || context.components != Component::Db2.bit() | Component::Collision.bit()
        || plan.map_domain.is_some()
    {
        return Err("product_plan_component_scope".into());
    }
    extract_inner(
        store,
        run,
        preflight,
        context,
        budget,
        GeometryPolicy::new_with_empty_wmo(
            MapSelection::All,
            run::MissingWdtPolicy::DeclareRootAbsent,
            run::MissingModelPolicy::DeclareRootAbsentWmoDoodadM2,
            resources::Profile::RequestHeavyV1,
        ),
    )
}

pub(crate) fn extract_v35(
    plan: &crate::plan::Plan,
    store: &mut forever_casc::local_store::LocalStore,
    run: &mut Run,
    preflight: &Preflight,
    context: Context,
    budget: DiskBudget,
) -> Result<(), Error> {
    if !plan.is_v35()
        || context.build != 70450
        || context.components != Component::Db2.bit() | Component::Collision.bit()
        || plan.map_domain.is_some()
    {
        return Err("product_plan_component_scope".into());
    }
    extract_inner(
        store,
        run,
        preflight,
        context,
        budget,
        GeometryPolicy::new_with_unselected_wmo(
            MapSelection::All,
            run::MissingWdtPolicy::DeclareRootAbsent,
            run::MissingModelPolicy::DeclareRootAbsentWmoDoodadM2,
            resources::Profile::RequestHeavyV1,
        ),
    )
}

pub(crate) fn extract_v36(
    plan: &crate::plan::Plan,
    store: &mut forever_casc::local_store::LocalStore,
    run: &mut Run,
    preflight: &Preflight,
    context: Context,
    budget: DiskBudget,
) -> Result<(), Error> {
    if !plan.is_v36()
        || context.build != 70450
        || context.components != Component::Db2.bit() | Component::Collision.bit()
        || plan.map_domain.is_some()
    {
        return Err("product_plan_component_scope".into());
    }
    extract_inner(
        store,
        run,
        preflight,
        context,
        budget,
        GeometryPolicy::new_with_material_filtered_wmo(
            MapSelection::All,
            run::MissingWdtPolicy::DeclareRootAbsent,
            run::MissingModelPolicy::DeclareRootAbsentWmoDoodadM2,
            resources::Profile::RequestHeavyV1,
        ),
    )
}

pub(crate) fn extract_v37(
    plan: &crate::plan::Plan,
    store: &mut forever_casc::local_store::LocalStore,
    run: &mut Run,
    preflight: &Preflight,
    context: Context,
    budget: DiskBudget,
) -> Result<(), Error> {
    if !plan.is_v37()
        || context.build != 70450
        || context.components != Component::Db2.bit() | Component::Collision.bit()
        || plan.map_domain.is_some()
    {
        return Err("product_plan_component_scope".into());
    }
    extract_inner(
        store,
        run,
        preflight,
        context,
        budget,
        GeometryPolicy::new_with_dependency_heavy_material_filtered_wmo(
            MapSelection::All,
            run::MissingWdtPolicy::DeclareRootAbsent,
            run::MissingModelPolicy::DeclareRootAbsentWmoDoodadM2,
        ),
    )
}

pub(crate) fn extract_v38(
    plan: &crate::plan::Plan,
    store: &mut forever_casc::local_store::LocalStore,
    run: &mut Run,
    preflight: &Preflight,
    context: Context,
    budget: DiskBudget,
) -> Result<(), Error> {
    if !plan.is_v38()
        || context.build != 70450
        || context.components != Component::Db2.bit() | Component::Collision.bit()
        || plan.map_domain.is_some()
    {
        return Err("product_plan_component_scope".into());
    }
    extract_inner(
        store,
        run,
        preflight,
        context,
        budget,
        GeometryPolicy::new_with_material_filtered_wmo(
            MapSelection::All,
            run::MissingWdtPolicy::DeclareRootAbsent,
            run::MissingModelPolicy::DeclareRootAbsentWmoDoodadM2,
            resources::Profile::SourceHeavyV1,
        ),
    )
}

pub(crate) fn extract_v39(
    plan: &crate::plan::Plan,
    store: &mut forever_casc::local_store::LocalStore,
    run: &mut Run,
    preflight: &Preflight,
    context: Context,
    budget: DiskBudget,
) -> Result<(), Error> {
    if !plan.is_v39()
        || context.build != 70450
        || context.components != Component::Db2.bit() | Component::Collision.bit()
        || plan.map_domain.is_some()
    {
        return Err("product_plan_component_scope".into());
    }
    extract_inner(
        store,
        run,
        preflight,
        context,
        budget,
        GeometryPolicy::new_with_read_heavy_material_filtered_wmo(
            MapSelection::All,
            run::MissingWdtPolicy::DeclareRootAbsent,
            run::MissingModelPolicy::DeclareRootAbsentWmoDoodadM2,
        ),
    )
}

pub(crate) fn extract_v40(
    plan: &crate::plan::Plan,
    store: &mut forever_casc::local_store::LocalStore,
    run: &mut Run,
    preflight: &Preflight,
    context: Context,
    budget: DiskBudget,
) -> Result<(), Error> {
    if !plan.is_v40()
        || context.build != 70450
        || context.components != Component::Db2.bit() | Component::Collision.bit()
        || plan.map_domain.is_some()
    {
        return Err("product_plan_component_scope".into());
    }
    extract_inner(
        store,
        run,
        preflight,
        context,
        budget,
        GeometryPolicy::new_with_model_heavy_material_filtered_wmo(
            MapSelection::All,
            run::MissingWdtPolicy::DeclareRootAbsent,
            run::MissingModelPolicy::DeclareRootAbsentWmoDoodadM2,
        ),
    )
}

pub(crate) fn extract_v41(
    plan: &crate::plan::Plan,
    store: &mut forever_casc::local_store::LocalStore,
    run: &mut Run,
    preflight: &Preflight,
    context: Context,
    budget: DiskBudget,
) -> Result<(), Error> {
    if !plan.is_v41()
        || context.build != 70450
        || context.components != Component::Db2.bit() | Component::Collision.bit()
        || plan.map_domain.is_some()
    {
        return Err("product_plan_component_scope".into());
    }
    extract_inner(
        store,
        run,
        preflight,
        context,
        budget,
        GeometryPolicy::new_with_material_filtered_wmo(
            MapSelection::All,
            run::MissingWdtPolicy::DeclareRootAbsent,
            run::MissingModelPolicy::DeclareRootAbsentWmoDoodadM2,
            resources::Profile::SourceExpandedV1,
        ),
    )
}

pub(crate) fn extract_v42(
    plan: &crate::plan::Plan,
    store: &mut forever_casc::local_store::LocalStore,
    run: &mut Run,
    preflight: &Preflight,
    context: Context,
    budget: DiskBudget,
) -> Result<(), Error> {
    if !plan.is_v42()
        || context.build != 70450
        || context.components != Component::Db2.bit() | Component::Collision.bit()
        || plan.map_domain.is_some()
    {
        return Err("product_plan_component_scope".into());
    }
    extract_inner(
        store,
        run,
        preflight,
        context,
        budget,
        GeometryPolicy::new_with_material_filtered_wmo(
            MapSelection::All,
            run::MissingWdtPolicy::DeclareRootAbsent,
            run::MissingModelPolicy::DeclareRootAbsentWmoDoodadM2,
            resources::Profile::ModelOutputExpandedV1,
        ),
    )
}

pub(crate) fn extract_v43(
    plan: &crate::plan::Plan,
    store: &mut forever_casc::local_store::LocalStore,
    run: &mut Run,
    preflight: &Preflight,
    context: Context,
    budget: DiskBudget,
) -> Result<(), Error> {
    if !plan.is_v43()
        || context.build != 70450
        || context.components != Component::Db2.bit() | Component::Collision.bit()
        || plan.map_domain.is_some()
    {
        return Err("product_plan_component_scope".into());
    }
    extract_inner(
        store,
        run,
        preflight,
        context,
        budget,
        GeometryPolicy::new_with_material_filtered_wmo(
            MapSelection::All,
            run::MissingWdtPolicy::DeclareRootAbsent,
            run::MissingModelPolicy::DeclareRootAbsentWmoDoodadM2,
            resources::Profile::ModelBytesExpandedV1,
        ),
    )
}

pub(crate) fn extract_v44(
    plan: &crate::plan::Plan,
    store: &mut forever_casc::local_store::LocalStore,
    run: &mut Run,
    preflight: &Preflight,
    context: Context,
    budget: DiskBudget,
) -> Result<(), Error> {
    if !plan.is_v44()
        || context.build != 70450
        || context.components != Component::Db2.bit() | Component::Collision.bit()
        || plan.map_domain.is_some()
    {
        return Err("product_plan_component_scope".into());
    }
    extract_inner(
        store,
        run,
        preflight,
        context,
        budget,
        GeometryPolicy::new_with_material_filtered_wmo(
            MapSelection::All,
            run::MissingWdtPolicy::DeclareRootAbsent,
            run::MissingModelPolicy::DeclareRootAbsentWmoDoodadM2,
            resources::Profile::SourceExpandedV2,
        ),
    )
}

pub(crate) fn extract_v45(
    plan: &crate::plan::Plan,
    store: &mut forever_casc::local_store::LocalStore,
    run: &mut Run,
    preflight: &Preflight,
    context: Context,
    budget: DiskBudget,
) -> Result<(), Error> {
    if !plan.is_v45()
        || context.build != 70450
        || context.components != Component::Db2.bit() | Component::Collision.bit()
        || plan.map_domain.is_some()
    {
        return Err("product_plan_component_scope".into());
    }
    extract_inner(
        store,
        run,
        preflight,
        context,
        budget,
        GeometryPolicy::new_with_material_filtered_wmo(
            MapSelection::All,
            run::MissingWdtPolicy::DeclareRootAbsent,
            run::MissingModelPolicy::DeclareRootAbsentWmoDoodadM2,
            resources::Profile::PlacementExpandedV1,
        ),
    )
}

pub(crate) fn extract_v46(
    plan: &crate::plan::Plan,
    store: &mut forever_casc::local_store::LocalStore,
    run: &mut Run,
    preflight: &Preflight,
    context: Context,
    budget: DiskBudget,
) -> Result<(), Error> {
    if !plan.is_v46()
        || context.build != 70450
        || context.components != Component::Db2.bit() | Component::Collision.bit()
        || plan.map_domain.is_some()
    {
        return Err("product_plan_component_scope".into());
    }
    extract_inner(
        store,
        run,
        preflight,
        context,
        budget,
        GeometryPolicy::new_with_material_filtered_wmo(
            MapSelection::All,
            run::MissingWdtPolicy::DeclareRootAbsent,
            run::MissingModelPolicy::DeclareRootAbsentWmoDoodadM2,
            resources::Profile::AssemblyExpandedV1,
        ),
    )
}

pub(crate) fn extract_v47(
    plan: &crate::plan::Plan,
    store: &mut forever_casc::local_store::LocalStore,
    run: &mut Run,
    preflight: &Preflight,
    context: Context,
    budget: DiskBudget,
) -> Result<(), Error> {
    if !plan.is_v47()
        || context.build != 70450
        || context.components != Component::Db2.bit() | Component::Collision.bit()
        || plan.map_domain.is_some()
    {
        return Err("product_plan_component_scope".into());
    }
    extract_inner(
        store,
        run,
        preflight,
        context,
        budget,
        GeometryPolicy::new_with_material_filtered_wmo(
            MapSelection::All,
            run::MissingWdtPolicy::DeclareRootAbsent,
            run::MissingModelPolicy::DeclareRootAbsentWmoDoodadM2,
            resources::Profile::GameObjectExpandedV1,
        ),
    )
}

pub(crate) fn extract_v48(
    plan: &crate::plan::Plan,
    store: &mut forever_casc::local_store::LocalStore,
    run: &mut Run,
    preflight: &Preflight,
    context: Context,
    budget: DiskBudget,
) -> Result<(), Error> {
    if !plan.is_v48()
        || context.build != 70450
        || context.components != Component::Db2.bit() | Component::Collision.bit()
        || plan.map_domain.is_some()
    {
        return Err("product_plan_component_scope".into());
    }
    extract_inner(
        store,
        run,
        preflight,
        context,
        budget,
        GeometryPolicy::new_with_material_filtered_wmo(
            MapSelection::All,
            run::MissingWdtPolicy::DeclareRootAbsent,
            run::MissingModelPolicy::DeclareRootAbsentWmoDoodadM2,
            resources::Profile::GameObjectMissingRootExpandedV1,
        )
        .with_gameobject_missing_root(),
    )
}

pub(crate) fn extract_v49(
    plan: &crate::plan::Plan,
    store: &mut forever_casc::local_store::LocalStore,
    run: &mut Run,
    preflight: &Preflight,
    context: Context,
    budget: DiskBudget,
) -> Result<(), Error> {
    if !(plan.is_v49() || plan.is_v50())
        || context.build != 70450
        || context.components != Component::Db2.bit() | Component::Collision.bit()
        || plan.map_domain.is_some()
    {
        return Err("product_plan_component_scope".into());
    }
    extract_v49_geometry(store, run, preflight, context, budget)
}

pub(crate) fn extract_v51(
    plan: &crate::plan::Plan,
    store: &mut forever_casc::local_store::LocalStore,
    run: &mut Run,
    preflight: &Preflight,
    context: Context,
    budget: DiskBudget,
) -> Result<(), Error> {
    let components = Component::Db2.bit() | Component::Terrain.bit() | Component::Collision.bit();
    if !plan.is_v51()
        || context.build != 70450
        || context.components != plan.components
        || context.components != components
        || plan.map_domain.is_some()
    {
        return Err("product_plan_component_scope".into());
    }
    extract_v49_geometry(store, run, preflight, context, budget)
}

fn extract_v49_geometry(
    store: &mut forever_casc::local_store::LocalStore,
    run: &mut Run,
    preflight: &Preflight,
    context: Context,
    budget: DiskBudget,
) -> Result<(), Error> {
    extract_inner(
        store,
        run,
        preflight,
        context,
        budget,
        GeometryPolicy::new_with_material_filtered_wmo(
            MapSelection::All,
            run::MissingWdtPolicy::DeclareRootAbsent,
            run::MissingModelPolicy::DeclareRootAbsentWmoDoodadM2,
            resources::Profile::GameObjectRootAndEncryptedUnavailableExpandedV1,
        )
        .with_gameobject_root_or_encrypted_unavailable(),
    )
}

fn extract_inner(
    store: &mut forever_casc::local_store::LocalStore,
    run: &mut Run,
    preflight: &Preflight,
    context: Context,
    budget: DiskBudget,
    policy: GeometryPolicy,
) -> Result<(), Error> {
    let GeometryPolicy {
        selection,
        missing_wdt_policy,
        missing_m2_policy,
        missing_wmo_policy,
        resource_profile,
        allow_gameobject_missing_root,
        allow_gameobject_encrypted_unavailable,
    } = policy;
    preflight.verify_unchanged()?;
    let v42 = resource_profile == resources::Profile::ModelOutputExpandedV1;
    let v43 = resource_profile == resources::Profile::ModelBytesExpandedV1;
    let v44 = resource_profile == resources::Profile::SourceExpandedV2;
    let v45 = resource_profile == resources::Profile::PlacementExpandedV1;
    let v46 = resource_profile == resources::Profile::AssemblyExpandedV1;
    let v47 = resource_profile == resources::Profile::GameObjectExpandedV1;
    let v48 = resource_profile == resources::Profile::GameObjectMissingRootExpandedV1;
    let v49 =
        resource_profile == resources::Profile::GameObjectRootAndEncryptedUnavailableExpandedV1;
    let mut sink = Sink {
        run,
        models: BTreeMap::new(),
        placements: BTreeMap::new(),
        model_bytes: 0,
        model_limit: as_usize(budget.vmo_bytes)?.min(
            if v43 || v44 || v45 || v46 || v47 || v48 || v49 {
                512 * 1024 * 1024
            } else {
                MAX_STAGED_MODEL_BYTES
            },
        ),
        max_models: local_model_limit(resource_profile).0,
        model_count_limit: local_model_limit(resource_profile).1,
        model_bytes_limit: if v42 || v43 || v44 || v45 || v46 || v47 || v48 || v49 {
            MODEL_BYTES_LIMIT
        } else {
            LIMIT
        },
        placement_bytes: 0,
        placement_limit: staged_placement_limit(resource_profile, budget.placement_bytes)?,
        placement_map_limit: if v45 || v46 || v47 || v48 || v49 {
            placement::Limits::default()
                .max_bytes
                .min(placement::MAX_BYTES)
        } else {
            placement::Limits::default().max_bytes
        },
        placement_capacity: 0,
        placement_error_codes: placement_error_codes(resource_profile),
        failure: None,
    };
    let limits = geometry_limits(budget, resource_profile)?;
    let result = match (
        selection,
        missing_wdt_policy,
        missing_m2_policy,
        missing_wmo_policy,
        resource_profile,
    ) {
        (
            MapSelection::MapAndAncestors(id),
            run::MissingWdtPolicy::Reject,
            run::MissingModelPolicy::Reject,
            run::MissingWmoPolicy::Reject,
            resources::Profile::BoundedV1,
        ) => geometry::extract_map(store, id, &mut sink, limits),
        (
            MapSelection::All,
            run::MissingWdtPolicy::Reject,
            run::MissingModelPolicy::Reject,
            run::MissingWmoPolicy::Reject,
            resources::Profile::BoundedV1,
        ) => geometry::extract_all(store, &mut sink, limits),
        (
            MapSelection::All,
            run::MissingWdtPolicy::DeclareRootAbsent,
            run::MissingModelPolicy::Reject,
            run::MissingWmoPolicy::Reject,
            resources::Profile::BoundedV1,
        ) => geometry::extract_all_with_root_absence(store, &mut sink, limits),
        (
            MapSelection::All,
            run::MissingWdtPolicy::DeclareRootAbsent,
            run::MissingModelPolicy::DeclareRootAbsentWmoDoodadM2,
            run::MissingWmoPolicy::Reject,
            resources::Profile::BoundedV1,
        ) => geometry::extract_all_with_model_root_absence(store, &mut sink, limits),
        (
            MapSelection::All,
            run::MissingWdtPolicy::DeclareRootAbsent,
            run::MissingModelPolicy::DeclareRootAbsentWmoDoodadM2,
            run::MissingWmoPolicy::Reject,
            resources::Profile::RequestHeavyV1,
        ) => {
            geometry::extract_all_with_model_root_absence_request_capacity(store, &mut sink, limits)
        }
        (
            MapSelection::All,
            run::MissingWdtPolicy::DeclareRootAbsent,
            run::MissingModelPolicy::DeclareRootAbsentWmoDoodadM2,
            run::MissingWmoPolicy::DeclareVacatedAfterRootAbsentM2,
            resources::Profile::RequestHeavyV1,
        ) => geometry::extract_all_with_vacated_wmo(store, &mut sink, limits),
        (
            MapSelection::All,
            run::MissingWdtPolicy::DeclareRootAbsent,
            run::MissingModelPolicy::DeclareRootAbsentWmoDoodadM2,
            run::MissingWmoPolicy::DeclareVacatedAfterRootAbsentM2OrEmpty,
            resources::Profile::RequestHeavyV1,
        ) => geometry::extract_all_with_empty_wmo(store, &mut sink, limits),
        (
            MapSelection::All,
            run::MissingWdtPolicy::DeclareRootAbsent,
            run::MissingModelPolicy::DeclareRootAbsentWmoDoodadM2,
            run::MissingWmoPolicy::DeclareVacatedAfterRootAbsentM2OrEmptyOrUnselected,
            resources::Profile::RequestHeavyV1,
        ) => geometry::extract_all_with_unselected_wmo(store, &mut sink, limits),
        (
            MapSelection::All,
            run::MissingWdtPolicy::DeclareRootAbsent,
            run::MissingModelPolicy::DeclareRootAbsentWmoDoodadM2,
            run::MissingWmoPolicy::DeclareVacatedAfterRootAbsentM2OrEmptyOrUnselectedOrMaterialFiltered,
            resources::Profile::RequestHeavyV1,
        ) => geometry::extract_all_with_material_filtered_wmo(store, &mut sink, limits),
        (
            MapSelection::All,
            run::MissingWdtPolicy::DeclareRootAbsent,
            run::MissingModelPolicy::DeclareRootAbsentWmoDoodadM2,
            run::MissingWmoPolicy::DeclareVacatedAfterRootAbsentM2OrEmptyOrUnselectedOrMaterialFiltered,
            resources::Profile::DependencyHeavyV1,
        ) => geometry::extract_all_with_dependency_heavy_material_filtered_wmo(
            store,
            &mut sink,
            limits,
        ),
        (
            MapSelection::All,
            run::MissingWdtPolicy::DeclareRootAbsent,
            run::MissingModelPolicy::DeclareRootAbsentWmoDoodadM2,
            run::MissingWmoPolicy::DeclareVacatedAfterRootAbsentM2OrEmptyOrUnselectedOrMaterialFiltered,
            resources::Profile::SourceHeavyV1,
        ) => geometry::extract_all_with_source_heavy_material_filtered_wmo_70450(
            store,
            &mut sink,
            limits,
        ),
        (
            MapSelection::All,
            run::MissingWdtPolicy::DeclareRootAbsent,
            run::MissingModelPolicy::DeclareRootAbsentWmoDoodadM2,
            run::MissingWmoPolicy::DeclareVacatedAfterRootAbsentM2OrEmptyOrUnselectedOrMaterialFiltered,
            resources::Profile::ReadHeavyV1,
        ) => geometry::extract_all_with_read_heavy_material_filtered_wmo_70450(
            store,
            &mut sink,
            limits,
        ),
        (
            MapSelection::All,
            run::MissingWdtPolicy::DeclareRootAbsent,
            run::MissingModelPolicy::DeclareRootAbsentWmoDoodadM2,
            run::MissingWmoPolicy::DeclareVacatedAfterRootAbsentM2OrEmptyOrUnselectedOrMaterialFiltered,
            resources::Profile::ModelHeavyV1,
        ) => geometry::extract_all_with_model_heavy_material_filtered_wmo_70450(
            store,
            &mut sink,
            limits,
        ),
        (
            MapSelection::All,
            run::MissingWdtPolicy::DeclareRootAbsent,
            run::MissingModelPolicy::DeclareRootAbsentWmoDoodadM2,
            run::MissingWmoPolicy::DeclareVacatedAfterRootAbsentM2OrEmptyOrUnselectedOrMaterialFiltered,
            resources::Profile::SourceExpandedV1,
        ) => geometry::extract_all_with_source_expanded_material_filtered_wmo_70450(
            store,
            &mut sink,
            limits,
        ),
        (
            MapSelection::All,
            run::MissingWdtPolicy::DeclareRootAbsent,
            run::MissingModelPolicy::DeclareRootAbsentWmoDoodadM2,
            run::MissingWmoPolicy::DeclareVacatedAfterRootAbsentM2OrEmptyOrUnselectedOrMaterialFiltered,
            resources::Profile::ModelOutputExpandedV1,
        ) => geometry::extract_all_with_model_output_expanded_material_filtered_wmo_70450(
            store,
            &mut sink,
            limits,
        ),
        (
            MapSelection::All,
            run::MissingWdtPolicy::DeclareRootAbsent,
            run::MissingModelPolicy::DeclareRootAbsentWmoDoodadM2,
            run::MissingWmoPolicy::DeclareVacatedAfterRootAbsentM2OrEmptyOrUnselectedOrMaterialFiltered,
            resources::Profile::ModelBytesExpandedV1,
        ) => geometry::extract_all_with_model_bytes_expanded_material_filtered_wmo_70450(
            store,
            &mut sink,
            limits,
        ),
        (
            MapSelection::All,
            run::MissingWdtPolicy::DeclareRootAbsent,
            run::MissingModelPolicy::DeclareRootAbsentWmoDoodadM2,
            run::MissingWmoPolicy::DeclareVacatedAfterRootAbsentM2OrEmptyOrUnselectedOrMaterialFiltered,
            resources::Profile::SourceExpandedV2,
        ) => geometry::extract_all_with_source_expanded_v2_material_filtered_wmo_70450(
            store,
            &mut sink,
            limits,
        ),
        (
            MapSelection::All,
            run::MissingWdtPolicy::DeclareRootAbsent,
            run::MissingModelPolicy::DeclareRootAbsentWmoDoodadM2,
            run::MissingWmoPolicy::DeclareVacatedAfterRootAbsentM2OrEmptyOrUnselectedOrMaterialFiltered,
            resources::Profile::PlacementExpandedV1,
        ) => geometry::extract_all_with_placement_expanded_material_filtered_wmo_70450(
            store,
            &mut sink,
            limits,
        ),
        (
            MapSelection::All,
            run::MissingWdtPolicy::DeclareRootAbsent,
            run::MissingModelPolicy::DeclareRootAbsentWmoDoodadM2,
            run::MissingWmoPolicy::DeclareVacatedAfterRootAbsentM2OrEmptyOrUnselectedOrMaterialFiltered,
            resources::Profile::AssemblyExpandedV1,
        ) => geometry::extract_all_with_assembly_expanded_material_filtered_wmo_70450(
            store,
            &mut sink,
            limits,
        ),
        (
            MapSelection::All,
            run::MissingWdtPolicy::DeclareRootAbsent,
            run::MissingModelPolicy::DeclareRootAbsentWmoDoodadM2,
            run::MissingWmoPolicy::DeclareVacatedAfterRootAbsentM2OrEmptyOrUnselectedOrMaterialFiltered,
            resources::Profile::GameObjectExpandedV1,
        ) => geometry::extract_all_with_gameobject_expanded_material_filtered_wmo_70450(
            store,
            &mut sink,
            limits,
        ),
        (
            MapSelection::All,
            run::MissingWdtPolicy::DeclareRootAbsent,
            run::MissingModelPolicy::DeclareRootAbsentWmoDoodadM2,
            run::MissingWmoPolicy::DeclareVacatedAfterRootAbsentM2OrEmptyOrUnselectedOrMaterialFiltered,
            resources::Profile::GameObjectMissingRootExpandedV1,
        ) if allow_gameobject_missing_root && !allow_gameobject_encrypted_unavailable => {
            geometry::extract_all_with_gameobject_missing_root_material_filtered_wmo_70450(
                store,
                &mut sink,
                limits,
            )
        }
        (
            MapSelection::All,
            run::MissingWdtPolicy::DeclareRootAbsent,
            run::MissingModelPolicy::DeclareRootAbsentWmoDoodadM2,
            run::MissingWmoPolicy::DeclareVacatedAfterRootAbsentM2OrEmptyOrUnselectedOrMaterialFiltered,
            resources::Profile::GameObjectRootAndEncryptedUnavailableExpandedV1,
        ) if allow_gameobject_missing_root && allow_gameobject_encrypted_unavailable => {
            geometry::extract_all_with_gameobject_root_or_encrypted_unavailable_material_filtered_wmo_70450(
                store,
                &mut sink,
                limits,
            )
        }
        _ => return Err("product_plan_component_scope".into()),
    };
    let completed = match result {
        Ok(completed) => completed,
        Err(error) => {
            return Err(sink
                .failure
                .take()
                .unwrap_or_else(|| Error::from(error.code())));
        }
    };
    let report = completed.report().run;
    if report.build != 70450
        || report.wdt_resident_bytes as u64 > budget.wdt_bytes
        || report.cache_peak_bytes as u64 > budget.cache_bytes
        || report.model_raw_bytes > budget.raw_bytes
        || report.placement_bytes > budget.placement_bytes
    {
        return Err(LIMIT.into());
    }
    let active_maps =
        u64::try_from(completed.world().map_stats().active_maps).map_err(|_| LIMIT)?;
    let root_absences = u64::try_from(completed.world().root_absent().len()).map_err(|_| LIMIT)?;
    let valid_report = match (selection, missing_wdt_policy) {
        (MapSelection::MapAndAncestors(_), run::MissingWdtPolicy::Reject) => true,
        (MapSelection::All, run::MissingWdtPolicy::Reject) => {
            report.maps == report.wdt_files && report.maps == active_maps
        }
        (MapSelection::All, run::MissingWdtPolicy::DeclareRootAbsent) => {
            report.maps == active_maps
                && report.maps.checked_sub(report.wdt_files) == Some(root_absences)
        }
        _ => false,
    };
    if !valid_report {
        return Err("product_collision_coverage".into());
    }

    let mut domain = completed_domain(&completed, policy)?;
    domain.sort_unstable_by_key(|entry| (entry.depth, entry.id));
    let placement_errors = placement_error_codes(resource_profile);
    validate_domain_count(selection, domain.len(), placement_errors.domain_count)?;
    let mut ids = Vec::new();
    ids.try_reserve_exact(domain.len())
        .map_err(|_| Error::from("product_allocation"))?;
    ids.extend(
        domain
            .iter()
            .filter(|entry| !entry.root_absent)
            .map(|entry| entry.id),
    );
    ids.sort_unstable();
    if missing_wdt_policy == run::MissingWdtPolicy::DeclareRootAbsent {
        let present = domain.iter().filter(|entry| !entry.root_absent).count();
        if report.wdt_files != u64::try_from(present).map_err(|_| LIMIT)? {
            return Err("product_collision_coverage".into());
        }
    }
    validate_placement_domain(
        sink.placements.keys().copied(),
        &ids,
        placement_errors.domain,
    )?;
    let missing_m2 = completed.world().missing_m2();
    let source_missing_wmo = completed.world().missing_wmo();
    let source_empty_wmo = completed.world().empty_wmo();
    let version = coverage_version(policy)?;
    let source_unselected_wmo = if matches!(
        version,
        UNSELECTED_WMO_COVERAGE_VERSION
            | MATERIAL_FILTERED_WMO_COVERAGE_VERSION
            | DEPENDENCY_HEAVY_MATERIAL_FILTERED_WMO_COVERAGE_VERSION
            | SOURCE_HEAVY_MATERIAL_FILTERED_WMO_COVERAGE_VERSION
            | READ_HEAVY_MATERIAL_FILTERED_WMO_COVERAGE_VERSION
            | MODEL_HEAVY_MATERIAL_FILTERED_WMO_COVERAGE_VERSION
            | SOURCE_EXPANDED_MATERIAL_FILTERED_WMO_COVERAGE_VERSION
            | MODEL_BYTES_EXPANDED_MATERIAL_FILTERED_WMO_COVERAGE_VERSION
            | SOURCE_EXPANDED_V2_MATERIAL_FILTERED_WMO_COVERAGE_VERSION
            | PLACEMENT_EXPANDED_MATERIAL_FILTERED_WMO_COVERAGE_VERSION
            | ASSEMBLY_EXPANDED_MATERIAL_FILTERED_WMO_COVERAGE_VERSION
            | MODEL_OUTPUT_EXPANDED_MATERIAL_FILTERED_WMO_COVERAGE_VERSION
            | GAMEOBJECT_EXPANDED_MATERIAL_FILTERED_WMO_COVERAGE_VERSION
            | GAMEOBJECT_MISSING_ROOT_COVERAGE_VERSION
            | GAMEOBJECT_ROOT_ENCRYPTED_UNAVAILABLE_COVERAGE_VERSION
    ) {
        completed.world().unselected_wmo()
    } else {
        &[]
    };
    let source_material_filtered_wmo = if matches!(
        version,
        MATERIAL_FILTERED_WMO_COVERAGE_VERSION
            | DEPENDENCY_HEAVY_MATERIAL_FILTERED_WMO_COVERAGE_VERSION
            | SOURCE_HEAVY_MATERIAL_FILTERED_WMO_COVERAGE_VERSION
            | READ_HEAVY_MATERIAL_FILTERED_WMO_COVERAGE_VERSION
            | MODEL_HEAVY_MATERIAL_FILTERED_WMO_COVERAGE_VERSION
            | SOURCE_EXPANDED_MATERIAL_FILTERED_WMO_COVERAGE_VERSION
            | MODEL_BYTES_EXPANDED_MATERIAL_FILTERED_WMO_COVERAGE_VERSION
            | SOURCE_EXPANDED_V2_MATERIAL_FILTERED_WMO_COVERAGE_VERSION
            | PLACEMENT_EXPANDED_MATERIAL_FILTERED_WMO_COVERAGE_VERSION
            | ASSEMBLY_EXPANDED_MATERIAL_FILTERED_WMO_COVERAGE_VERSION
            | MODEL_OUTPUT_EXPANDED_MATERIAL_FILTERED_WMO_COVERAGE_VERSION
            | GAMEOBJECT_EXPANDED_MATERIAL_FILTERED_WMO_COVERAGE_VERSION
            | GAMEOBJECT_MISSING_ROOT_COVERAGE_VERSION
            | GAMEOBJECT_ROOT_ENCRYPTED_UNAVAILABLE_COVERAGE_VERSION
    ) {
        completed.world().material_filtered_wmo()
    } else {
        &[]
    };
    validate_missing_m2(missing_m2, missing_m2_policy)?;
    validate_missing_wmo(source_missing_wmo, missing_wmo_policy, missing_m2)?;
    validate_empty_wmo(
        source_empty_wmo,
        missing_wmo_policy,
        source_missing_wmo,
        missing_m2,
    )?;
    if matches!(
        version,
        UNSELECTED_WMO_COVERAGE_VERSION
            | MATERIAL_FILTERED_WMO_COVERAGE_VERSION
            | DEPENDENCY_HEAVY_MATERIAL_FILTERED_WMO_COVERAGE_VERSION
            | SOURCE_HEAVY_MATERIAL_FILTERED_WMO_COVERAGE_VERSION
            | READ_HEAVY_MATERIAL_FILTERED_WMO_COVERAGE_VERSION
            | MODEL_HEAVY_MATERIAL_FILTERED_WMO_COVERAGE_VERSION
            | SOURCE_EXPANDED_MATERIAL_FILTERED_WMO_COVERAGE_VERSION
            | MODEL_BYTES_EXPANDED_MATERIAL_FILTERED_WMO_COVERAGE_VERSION
            | SOURCE_EXPANDED_V2_MATERIAL_FILTERED_WMO_COVERAGE_VERSION
            | PLACEMENT_EXPANDED_MATERIAL_FILTERED_WMO_COVERAGE_VERSION
            | ASSEMBLY_EXPANDED_MATERIAL_FILTERED_WMO_COVERAGE_VERSION
            | MODEL_OUTPUT_EXPANDED_MATERIAL_FILTERED_WMO_COVERAGE_VERSION
            | GAMEOBJECT_EXPANDED_MATERIAL_FILTERED_WMO_COVERAGE_VERSION
            | GAMEOBJECT_MISSING_ROOT_COVERAGE_VERSION
            | GAMEOBJECT_ROOT_ENCRYPTED_UNAVAILABLE_COVERAGE_VERSION
    ) {
        validate_unselected_wmo(
            source_unselected_wmo,
            missing_wmo_policy,
            missing_m2,
            source_missing_wmo,
            source_empty_wmo,
        )?;
    }
    validate_material_filtered_wmo(
        source_material_filtered_wmo,
        missing_wmo_policy,
        missing_m2,
        source_missing_wmo,
        source_empty_wmo,
        source_unselected_wmo,
    )?;
    let mut missing_wmo = Vec::new();
    missing_wmo
        .try_reserve_exact(source_missing_wmo.len())
        .map_err(|_| Error::from("product_allocation"))?;
    missing_wmo.extend_from_slice(source_missing_wmo);
    let mut empty_wmo = Vec::new();
    empty_wmo
        .try_reserve_exact(source_empty_wmo.len())
        .map_err(|_| Error::from("product_allocation"))?;
    empty_wmo.extend_from_slice(source_empty_wmo);
    let mut unselected_wmo = Vec::new();
    if matches!(
        version,
        UNSELECTED_WMO_COVERAGE_VERSION
            | MATERIAL_FILTERED_WMO_COVERAGE_VERSION
            | DEPENDENCY_HEAVY_MATERIAL_FILTERED_WMO_COVERAGE_VERSION
            | SOURCE_HEAVY_MATERIAL_FILTERED_WMO_COVERAGE_VERSION
            | READ_HEAVY_MATERIAL_FILTERED_WMO_COVERAGE_VERSION
            | MODEL_HEAVY_MATERIAL_FILTERED_WMO_COVERAGE_VERSION
            | SOURCE_EXPANDED_MATERIAL_FILTERED_WMO_COVERAGE_VERSION
            | MODEL_BYTES_EXPANDED_MATERIAL_FILTERED_WMO_COVERAGE_VERSION
            | SOURCE_EXPANDED_V2_MATERIAL_FILTERED_WMO_COVERAGE_VERSION
            | PLACEMENT_EXPANDED_MATERIAL_FILTERED_WMO_COVERAGE_VERSION
            | ASSEMBLY_EXPANDED_MATERIAL_FILTERED_WMO_COVERAGE_VERSION
            | MODEL_OUTPUT_EXPANDED_MATERIAL_FILTERED_WMO_COVERAGE_VERSION
            | GAMEOBJECT_EXPANDED_MATERIAL_FILTERED_WMO_COVERAGE_VERSION
            | GAMEOBJECT_MISSING_ROOT_COVERAGE_VERSION
            | GAMEOBJECT_ROOT_ENCRYPTED_UNAVAILABLE_COVERAGE_VERSION
    ) {
        unselected_wmo
            .try_reserve_exact(source_unselected_wmo.len())
            .map_err(|_| Error::from("product_allocation"))?;
        unselected_wmo.extend_from_slice(source_unselected_wmo);
    }
    let mut material_filtered_wmo = Vec::new();
    if matches!(
        version,
        MATERIAL_FILTERED_WMO_COVERAGE_VERSION
            | DEPENDENCY_HEAVY_MATERIAL_FILTERED_WMO_COVERAGE_VERSION
            | SOURCE_HEAVY_MATERIAL_FILTERED_WMO_COVERAGE_VERSION
            | READ_HEAVY_MATERIAL_FILTERED_WMO_COVERAGE_VERSION
            | MODEL_HEAVY_MATERIAL_FILTERED_WMO_COVERAGE_VERSION
            | SOURCE_EXPANDED_MATERIAL_FILTERED_WMO_COVERAGE_VERSION
            | MODEL_BYTES_EXPANDED_MATERIAL_FILTERED_WMO_COVERAGE_VERSION
            | SOURCE_EXPANDED_V2_MATERIAL_FILTERED_WMO_COVERAGE_VERSION
            | PLACEMENT_EXPANDED_MATERIAL_FILTERED_WMO_COVERAGE_VERSION
            | ASSEMBLY_EXPANDED_MATERIAL_FILTERED_WMO_COVERAGE_VERSION
            | MODEL_OUTPUT_EXPANDED_MATERIAL_FILTERED_WMO_COVERAGE_VERSION
            | GAMEOBJECT_EXPANDED_MATERIAL_FILTERED_WMO_COVERAGE_VERSION
            | GAMEOBJECT_MISSING_ROOT_COVERAGE_VERSION
            | GAMEOBJECT_ROOT_ENCRYPTED_UNAVAILABLE_COVERAGE_VERSION
    ) {
        material_filtered_wmo
            .try_reserve_exact(source_material_filtered_wmo.len())
            .map_err(|_| Error::from("product_allocation"))?;
        material_filtered_wmo.extend_from_slice(source_material_filtered_wmo);
    }

    let mut artifacts = BTreeMap::new();
    let mut model_total = 0_u64;
    let mut map_total = 0_u64;
    let map_total_limit = assembled_domain_limit(resource_profile, budget.map_bytes);
    let map_total_error = assembled_domain_error(resource_profile);
    let mut parent_payloads = BTreeMap::<(u32, u32, u32), Vec<u8>>::new();
    let mut parent_payload_bytes = 0usize;
    let mut parent_payload_capacity = 0usize;
    let mut used_models = std::collections::BTreeMap::<String, ()>::new();
    if v47 || v48 || v49 {
        publish_gameobject_tree(
            &completed,
            &mut sink,
            preflight,
            &mut artifacts,
            &mut used_models,
        )?;
    }
    let mut placement_limits = placement::Limits::default();
    placement_limits.max_bytes = placement_limits.max_bytes.min(placement::MAX_BYTES);
    placement_limits.max_records = placement_limits.max_records.min(MAX_PLACEMENT_RECORDS);
    for map in &domain {
        if map.root_absent {
            if sink.placements.contains_key(&map.id) {
                return Err(placement_errors.domain.into());
            }
            continue;
        }
        let empty = Vec::new();
        let bytes = sink.placements.get(&map.id).unwrap_or(&empty);
        let parsed = placement::DirBin::parse(bytes, placement_limits)
            .map_err(|error| Error::from(error.code()))?;
        if parsed.record_count() > MAX_PLACEMENT_RECORDS {
            return Err(LIMIT.into());
        }
        let mut records = Vec::new();
        records
            .try_reserve_exact(parsed.record_count())
            .map_err(|_| Error::from("product_allocation"))?;
        for record in parsed.records() {
            let name = str::from_utf8(record.name())
                .map_err(|_| Error::from("product_selected_geometry_model"))?;
            if is_omitted_wmo_name(
                name,
                missing_m2,
                &missing_wmo,
                &empty_wmo,
                &unselected_wmo,
                &material_filtered_wmo,
                matches!(
                    version,
                    UNSELECTED_WMO_COVERAGE_VERSION
                        | MATERIAL_FILTERED_WMO_COVERAGE_VERSION
                        | DEPENDENCY_HEAVY_MATERIAL_FILTERED_WMO_COVERAGE_VERSION
                        | SOURCE_HEAVY_MATERIAL_FILTERED_WMO_COVERAGE_VERSION
                        | READ_HEAVY_MATERIAL_FILTERED_WMO_COVERAGE_VERSION
                        | MODEL_HEAVY_MATERIAL_FILTERED_WMO_COVERAGE_VERSION
                        | SOURCE_EXPANDED_MATERIAL_FILTERED_WMO_COVERAGE_VERSION
                        | MODEL_BYTES_EXPANDED_MATERIAL_FILTERED_WMO_COVERAGE_VERSION
                        | SOURCE_EXPANDED_V2_MATERIAL_FILTERED_WMO_COVERAGE_VERSION
                        | PLACEMENT_EXPANDED_MATERIAL_FILTERED_WMO_COVERAGE_VERSION
                        | ASSEMBLY_EXPANDED_MATERIAL_FILTERED_WMO_COVERAGE_VERSION
                        | MODEL_OUTPUT_EXPANDED_MATERIAL_FILTERED_WMO_COVERAGE_VERSION
                        | GAMEOBJECT_EXPANDED_MATERIAL_FILTERED_WMO_COVERAGE_VERSION
                        | GAMEOBJECT_MISSING_ROOT_COVERAGE_VERSION
                        | GAMEOBJECT_ROOT_ENCRYPTED_UNAVAILABLE_COVERAGE_VERSION
                ),
            ) {
                return Err(SCHEMA.into());
            }
            let model = sink
                .models
                .get(name)
                .ok_or("product_selected_geometry_model_binding")?;
            used_models.insert(name.to_owned(), ());
            records.push(
                placement::bounds::complete(record, model.bounds, placement_limits)
                    .map_err(|error| Error::from(error.code()))?,
            );
        }
        let mut limits = assembly::Limits::default();
        limits.max_memory_bytes = limits.max_memory_bytes.min(MAX_ASSEMBLY_MEMORY_BYTES);
        limits.max_input_bytes = limits.max_input_bytes.min(placement::MAX_BYTES);
        let remaining_after_proof =
            assembled_output_allowance(map_total_limit, map_total, bytes.len(), map_total_error)
                .map_err(Error::from)?;
        limits.max_output_bytes = limits
            .max_output_bytes
            .min(MAX_ASSEMBLY_OUTPUT_BYTES)
            .min(remaining_after_proof);
        limits.max_spawns = limits.max_spawns.min(MAX_PLACEMENT_RECORDS);
        let assembled =
            assembly::build(map.id, &records, limits).map_err(|error| Error::from(error.code()))?;
        let (tree_path, proof_path) = map_paths(map.id);
        let proof = assembled.ordered_dir_bin();
        for tile in assembled.tiles() {
            let index = tile.index_bytes();
            if let Some(payload) = tile.payload() {
                assembly::verify_associations(payload, index, proof, limits)
                    .map_err(|error| Error::from(error.code()))?;
                if map.is_parent {
                    let next = next_parent_payload_bytes(
                        parent_payload_bytes,
                        payload.len(),
                        map_total_limit,
                        map_total_error,
                    )
                    .map_err(Error::from)?;
                    let mut retained = Vec::new();
                    retained
                        .try_reserve_exact(payload.len())
                        .map_err(|_| Error::from("product_allocation"))?;
                    parent_payload_capacity = next_parent_payload_capacity(
                        parent_payload_capacity,
                        retained.capacity(),
                        as_usize(map_total_limit)?,
                        map_total_error,
                    )
                    .map_err(Error::from)?;
                    retained.extend_from_slice(payload);
                    if parent_payloads
                        .insert((map.id, tile.x(), tile.y()), retained)
                        .is_some()
                    {
                        return Err("product_selected_geometry_artifact_alias".into());
                    }
                    parent_payload_bytes = next;
                }
            } else {
                let mut ancestor = map.parent;
                let mut actual = None;
                while let Some(ancestor_id) = ancestor {
                    let ancestor_map = domain
                        .iter()
                        .find(|entry| entry.id == ancestor_id)
                        .ok_or(placement_errors.domain)?;
                    if ancestor_map.root_absent {
                        return Err("product_selected_geometry_parent_payload".into());
                    }
                    if let Some(bytes) = parent_payloads.get(&(ancestor_id, tile.x(), tile.y())) {
                        actual = Some(bytes.as_slice());
                        break;
                    }
                    ancestor = ancestor_map.parent;
                }
                assembly::verify_inherited_associations(
                    actual.ok_or("product_selected_geometry_parent_payload")?,
                    index,
                    proof,
                    limits,
                )
                .map_err(|error| Error::from(error.code()))?;
            }
        }
        let mut size =
            add_assembled_domain_bytes(0, proof.len(), map_total_error).map_err(Error::from)?;
        size = add_assembled_domain_bytes(size, assembled.tree_bytes().len(), map_total_error)
            .map_err(Error::from)?;
        for tile in assembled.tiles() {
            size = add_assembled_domain_bytes(size, tile.index_bytes().len(), map_total_error)
                .map_err(Error::from)?;
            if let Some(payload) = tile.payload() {
                size = add_assembled_domain_bytes(size, payload.len(), map_total_error)
                    .map_err(Error::from)?;
            }
        }
        map_total = next_assembled_domain_total(map_total, size, map_total_limit, map_total_error)
            .map_err(Error::from)?;
        publish(sink.run, preflight, &mut artifacts, proof_path, proof)?;
        publish(
            sink.run,
            preflight,
            &mut artifacts,
            tree_path,
            assembled.tree_bytes(),
        )?;
        for tile in assembled.tiles() {
            publish(
                sink.run,
                preflight,
                &mut artifacts,
                tile_path(map.id, tile.x(), tile.y(), true),
                tile.index_bytes(),
            )?;
            if let Some(payload) = tile.payload() {
                publish(
                    sink.run,
                    preflight,
                    &mut artifacts,
                    tile_path(map.id, tile.x(), tile.y(), false),
                    payload,
                )?;
            }
        }
    }
    for name in used_models.keys() {
        if is_omitted_wmo_name(
            name,
            missing_m2,
            &missing_wmo,
            &empty_wmo,
            &unselected_wmo,
            &material_filtered_wmo,
            matches!(
                version,
                UNSELECTED_WMO_COVERAGE_VERSION
                    | MATERIAL_FILTERED_WMO_COVERAGE_VERSION
                    | DEPENDENCY_HEAVY_MATERIAL_FILTERED_WMO_COVERAGE_VERSION
                    | SOURCE_HEAVY_MATERIAL_FILTERED_WMO_COVERAGE_VERSION
                    | READ_HEAVY_MATERIAL_FILTERED_WMO_COVERAGE_VERSION
                    | MODEL_HEAVY_MATERIAL_FILTERED_WMO_COVERAGE_VERSION
                    | SOURCE_EXPANDED_MATERIAL_FILTERED_WMO_COVERAGE_VERSION
                    | MODEL_BYTES_EXPANDED_MATERIAL_FILTERED_WMO_COVERAGE_VERSION
                    | SOURCE_EXPANDED_V2_MATERIAL_FILTERED_WMO_COVERAGE_VERSION
                    | PLACEMENT_EXPANDED_MATERIAL_FILTERED_WMO_COVERAGE_VERSION
                    | ASSEMBLY_EXPANDED_MATERIAL_FILTERED_WMO_COVERAGE_VERSION
                    | MODEL_OUTPUT_EXPANDED_MATERIAL_FILTERED_WMO_COVERAGE_VERSION
                    | GAMEOBJECT_EXPANDED_MATERIAL_FILTERED_WMO_COVERAGE_VERSION
                    | GAMEOBJECT_MISSING_ROOT_COVERAGE_VERSION
                    | GAMEOBJECT_ROOT_ENCRYPTED_UNAVAILABLE_COVERAGE_VERSION
            ),
        ) {
            return Err(SCHEMA.into());
        }
        let staged = sink
            .models
            .get(name)
            .ok_or("product_selected_geometry_model_binding")?;
        model_total = model_total
            .checked_add(staged.bytes.len() as u64)
            .ok_or(LIMIT)?;
        if model_total > budget.vmo_bytes {
            return Err(LIMIT.into());
        }
        publish(
            sink.run,
            preflight,
            &mut artifacts,
            format!("vmaps/{name}.vmo"),
            &staged.bytes,
        )?;
    }
    if artifacts.len() > COVERAGE_MAX_ARTIFACTS {
        return Err(LIMIT.into());
    }
    let coverage = Coverage::from_completed(
        &completed,
        context,
        policy,
        completed.coverage().gameobject_file_data_id,
        &used_models,
        &artifacts,
    )?;
    let bytes = coverage.encode()?;
    let coverage_path = if context.components == 25 {
        DB2_TERRAIN_LIQUID_VMAP_COVERAGE_PATH
    } else {
        coverage_path(policy)?
    };
    preflight.verify_unchanged()?;
    sink.run.put(Component::Collision, coverage_path, &bytes)?;
    store
        .verify_unchanged()
        .map_err(|error| Error::from(error.code()))?;
    preflight.verify_unchanged()?;
    Ok(())
}
