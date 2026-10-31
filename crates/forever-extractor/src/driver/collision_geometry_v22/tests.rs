// SPDX-License-Identifier: GPL-3.0-or-later
// Modified by the RustyCore Extractors project, 2026-10-19.
//! Synthetic v22-v47 producer-coverage and geometry-limit checks; no client files are used.
use super::*;
use forever_casc::Locale;
#[cfg(target_os = "linux")]
use forever_vmap::wmo::liquid::LiquidLookup;
#[cfg(target_os = "linux")]
#[path = "../../../tests/v39_read_heavy_material_filtered_wmo_geometry_cli/fixture.rs"]
#[allow(dead_code)]
#[allow(clippy::duplicate_mod)]
mod model_staging_fixture;

#[cfg(target_os = "linux")]
struct Lookup;
#[cfg(target_os = "linux")]
impl LiquidLookup for Lookup {
    fn material_id(&self, _: u32) -> Option<u8> {
        None
    }

    fn material_flags(&self, _: u8) -> Option<u32> {
        None
    }
}

fn sample() -> Coverage {
    Coverage {
        context: Context {
            build: 70450,
            locale: Locale::EnUs,
            components: Component::Collision.bit(),
            source_sha256: [0x31; 32],
            plan_sha256: [0x32; 32],
            producer_sha256: [0x33; 32],
        },
        selection: MapSelection::MapAndAncestors(2),
        missing_wdt_policy: run::MissingWdtPolicy::Reject,
        missing_m2_policy: run::MissingModelPolicy::Reject,
        missing_wmo_policy: run::MissingWmoPolicy::Reject,
        resource_profile: resources::Profile::BoundedV1,
        missing_m2: Vec::new(),
        missing_wmo: Vec::new(),
        empty_wmo: Vec::new(),
        unselected_wmo: Vec::new(),
        material_filtered_wmo: Vec::new(),
        gameobject_missing_root: Vec::new(),
        gameobject_encrypted_unavailable: Vec::new(),
        gameobject_scope: geometry::GameObjectScope::Excluded,
        gameobject_file_data_id: forever_casc::collision::gameobjects::TABLE_FILE_DATA_ID,
        gameobject_block_reason: geometry::GameObjectBlockReason::ExcludedByProfile,
        gameobject_request_made: false,
        gameobject_startup_reads: 0,
        gameobjects: None,
        model_scope: ModelScope::PlacementOnly,
        liquid_object_scope: geometry::LiquidObjectScope::OutOfScope,
        liquid_object_request_made: false,
        map: AvailableMapFact {
            file_data_id: world::MAP_FILE_DATA_ID,
            declared_bytes: 204,
            available_bytes: 204,
            full_ckey_verified: true,
            encoded_key_authenticated: true,
            declared_direct_rows: 0,
            declared_copy_records: 0,
            direct_rows: 0,
            copy_records: 0,
            materialized_rows: 0,
            sections: vec![SectionFact {
                ordinal: 0,
                records: 0,
                copies: 0,
                admitted: true,
            }],
        },
        liquids: [
            LiquidFact {
                file_data_id: world::MATERIAL_FILE_DATA_ID,
                bytes: 4,
                magic: *b"MATL",
                direct_rows: 0,
                copy_records: 0,
                materialized_rows: 0,
            },
            LiquidFact {
                file_data_id: world::TYPE_FILE_DATA_ID,
                bytes: 4,
                magic: *b"TYPE",
                direct_rows: 0,
                copy_records: 0,
                materialized_rows: 0,
            },
        ],
        maps: vec![DomainMapFact {
            id: 2,
            parent: None,
            wdt_file_data_id: 100,
            depth: 0,
            is_parent: false,
            root_absent: false,
        }],
        artifacts: vec![ArtifactFact {
            path: "vmaps/0002/0002.vmtree".into(),
            bytes: 16,
            sha: [0x34; 32],
        }],
    }
}

fn available_sample() -> Coverage {
    let mut coverage = sample();
    coverage.selection = MapSelection::All;
    coverage.context.components = Component::Db2.bit() | Component::Collision.bit();
    coverage
}

fn root_absent_sample() -> Coverage {
    let mut coverage = available_sample();
    coverage.missing_wdt_policy = run::MissingWdtPolicy::DeclareRootAbsent;
    coverage.maps = vec![
        DomainMapFact {
            id: 1,
            parent: None,
            wdt_file_data_id: 99,
            depth: 0,
            is_parent: true,
            root_absent: false,
        },
        DomainMapFact {
            id: 2,
            parent: Some(1),
            wdt_file_data_id: 100,
            depth: 1,
            is_parent: true,
            root_absent: true,
        },
        DomainMapFact {
            id: 3,
            parent: Some(2),
            wdt_file_data_id: 101,
            depth: 2,
            is_parent: false,
            root_absent: false,
        },
        DomainMapFact {
            id: 5,
            parent: Some(2),
            wdt_file_data_id: 101,
            depth: 2,
            is_parent: false,
            root_absent: true,
        },
    ];
    coverage.artifacts.clear();
    coverage
}

fn model_root_absent_sample() -> Coverage {
    let mut coverage = root_absent_sample();
    coverage.missing_m2_policy = run::MissingModelPolicy::DeclareRootAbsentWmoDoodadM2;
    coverage.missing_m2 = vec![0x1122_3344, 0x5566_7788];
    coverage
}

fn request_capacity_sample() -> Coverage {
    let mut coverage = model_root_absent_sample();
    coverage.resource_profile = resources::Profile::RequestHeavyV1;
    coverage
}

fn vacated_wmo_sample() -> Coverage {
    let mut coverage = request_capacity_sample();
    coverage.missing_wmo_policy = run::MissingWmoPolicy::DeclareVacatedAfterRootAbsentM2;
    coverage.missing_wmo = vec![0xAABB_CCDD, 0xDDEE_FF00];
    coverage
}

fn empty_wmo_sample() -> Coverage {
    let mut coverage = request_capacity_sample();
    coverage.missing_wmo_policy = run::MissingWmoPolicy::DeclareVacatedAfterRootAbsentM2OrEmpty;
    coverage.missing_wmo = vec![0xAABB_CCDD, 0xDDEE_FF00];
    coverage.empty_wmo = vec![0x1234_5678, 0x8765_4321];
    coverage
}

fn unselected_wmo_sample() -> Coverage {
    let mut coverage = request_capacity_sample();
    coverage.missing_wmo_policy =
        run::MissingWmoPolicy::DeclareVacatedAfterRootAbsentM2OrEmptyOrUnselected;
    coverage.missing_wmo = vec![0xAABB_CCDD, 0xDDEE_FF00];
    coverage.empty_wmo = vec![0x1234_5678, 0x8765_4321];
    coverage.unselected_wmo = vec![0x0102_0304, 0x1020_3040];
    coverage
}

fn unselected_wmo_policy() -> GeometryPolicy {
    GeometryPolicy::new_with_unselected_wmo(
        MapSelection::All,
        run::MissingWdtPolicy::DeclareRootAbsent,
        run::MissingModelPolicy::DeclareRootAbsentWmoDoodadM2,
        resources::Profile::RequestHeavyV1,
    )
}

fn material_filtered_wmo_sample() -> Coverage {
    let mut coverage = unselected_wmo_sample();
    coverage.missing_wmo_policy =
        run::MissingWmoPolicy::DeclareVacatedAfterRootAbsentM2OrEmptyOrUnselectedOrMaterialFiltered;
    coverage.material_filtered_wmo = vec![0x0102_0305, 0x1020_3041];
    coverage
}

fn material_filtered_wmo_policy() -> GeometryPolicy {
    GeometryPolicy::new_with_material_filtered_wmo(
        MapSelection::All,
        run::MissingWdtPolicy::DeclareRootAbsent,
        run::MissingModelPolicy::DeclareRootAbsentWmoDoodadM2,
        resources::Profile::RequestHeavyV1,
    )
}

fn dependency_heavy_material_filtered_wmo_sample() -> Coverage {
    let mut coverage = material_filtered_wmo_sample();
    coverage.resource_profile = resources::Profile::DependencyHeavyV1;
    coverage
}

fn dependency_heavy_material_filtered_wmo_policy() -> GeometryPolicy {
    GeometryPolicy::new_with_dependency_heavy_material_filtered_wmo(
        MapSelection::All,
        run::MissingWdtPolicy::DeclareRootAbsent,
        run::MissingModelPolicy::DeclareRootAbsentWmoDoodadM2,
    )
}

fn source_heavy_material_filtered_wmo_sample() -> Coverage {
    let mut coverage = material_filtered_wmo_sample();
    coverage.resource_profile = resources::Profile::SourceHeavyV1;
    coverage
}

fn source_heavy_material_filtered_wmo_policy() -> GeometryPolicy {
    GeometryPolicy::new_with_material_filtered_wmo(
        MapSelection::All,
        run::MissingWdtPolicy::DeclareRootAbsent,
        run::MissingModelPolicy::DeclareRootAbsentWmoDoodadM2,
        resources::Profile::SourceHeavyV1,
    )
}

fn read_heavy_material_filtered_wmo_sample() -> Coverage {
    let mut coverage = material_filtered_wmo_sample();
    coverage.resource_profile = resources::Profile::ReadHeavyV1;
    coverage
}

fn model_heavy_material_filtered_wmo_sample() -> Coverage {
    let mut coverage = material_filtered_wmo_sample();
    coverage.resource_profile = resources::Profile::ModelHeavyV1;
    coverage
}

fn source_expanded_material_filtered_wmo_sample() -> Coverage {
    let mut coverage = material_filtered_wmo_sample();
    coverage.resource_profile = resources::Profile::SourceExpandedV1;
    coverage
}

fn model_output_expanded_material_filtered_wmo_sample() -> Coverage {
    let mut coverage = material_filtered_wmo_sample();
    coverage.resource_profile = resources::Profile::ModelOutputExpandedV1;
    coverage
}

fn model_bytes_expanded_material_filtered_wmo_sample() -> Coverage {
    let mut coverage = material_filtered_wmo_sample();
    coverage.resource_profile = resources::Profile::ModelBytesExpandedV1;
    coverage
}

fn source_expanded_v2_material_filtered_wmo_sample() -> Coverage {
    let mut coverage = material_filtered_wmo_sample();
    coverage.resource_profile = resources::Profile::SourceExpandedV2;
    coverage
}

fn placement_expanded_material_filtered_wmo_sample() -> Coverage {
    let mut coverage = material_filtered_wmo_sample();
    coverage.resource_profile = resources::Profile::PlacementExpandedV1;
    coverage
}

fn assembly_expanded_material_filtered_wmo_sample() -> Coverage {
    let mut coverage = material_filtered_wmo_sample();
    coverage.resource_profile = resources::Profile::AssemblyExpandedV1;
    coverage
}

fn gameobject_expanded_material_filtered_wmo_sample() -> Coverage {
    let mut coverage = material_filtered_wmo_sample();
    coverage.resource_profile = resources::Profile::GameObjectExpandedV1;
    coverage.gameobject_scope = geometry::GameObjectScope::AvailableFdidDirectV1;
    coverage.gameobject_block_reason = geometry::GameObjectBlockReason::None;
    coverage.gameobject_request_made = true;
    coverage.gameobject_startup_reads = 1;
    coverage.model_scope = ModelScope::PlacementAndGameObject;
    coverage.gameobjects = Some(GameObjectFacts {
        declared_bytes: 512,
        available_bytes: 512,
        full_ckey_verified: true,
        encoded_key_authenticated: true,
        declared_direct_rows: 4,
        declared_copy_records: 1,
        direct_rows: 4,
        copy_records: 1,
        materialized_rows: 4,
        candidate_requests: 3,
        direct_outcomes: 4,
        zero_fdids: 1,
        textures: 1,
        m2_no_collision: 0,
        wmo_doodad_only: 0,
        missing_root_rows: 0,
        missing_root_fdids: Vec::new(),
        encrypted_unavailable_rows: 0,
        encrypted_unavailable_fdids: Vec::new(),
        collision_rows: 2,
        list_rows: 2,
        list_m2_rows: 2,
        list_wmo_rows: 0,
        list_vertex_visits: 8,
        list_raw_visit_bytes: 256,
        temp_bytes: 8 + 2 * (8 + 16),
        dtree_bytes: 8 + 2 * (8 + 16 + 24),
        temp_sha: [0x51; 32],
        dtree_sha: [0x52; 32],
        work_strict_candidates: 3,
        work_partial_candidates: 0,
        work_partial_bytes: 0,
        sections: vec![GameObjectSectionFact {
            ordinal: 0,
            records: 4,
            copies: 1,
            admitted: true,
        }],
        outcomes: vec![
            GameObjectOutcomeFact {
                ordinal: 0,
                display_id: 10,
                file_data_id: 0,
                disposition: GameObjectDisposition::ZeroFileDataId,
            },
            GameObjectOutcomeFact {
                ordinal: 1,
                display_id: 11,
                file_data_id: 200,
                disposition: GameObjectDisposition::Texture,
            },
            GameObjectOutcomeFact {
                ordinal: 2,
                display_id: 12,
                file_data_id: 201,
                disposition: GameObjectDisposition::Collision,
            },
            GameObjectOutcomeFact {
                ordinal: 3,
                display_id: 13,
                file_data_id: 202,
                disposition: GameObjectDisposition::Collision,
            },
        ],
    });
    coverage.artifacts.extend([
        ArtifactFact {
            path: "vmaps/0002/0002.vmtree".into(),
            bytes: 16,
            sha: [0x34; 32],
        },
        ArtifactFact {
            path: "vmaps/FILE000000C9.xxx.vmo".into(),
            bytes: 1,
            sha: [0x53; 32],
        },
        ArtifactFact {
            path: "vmaps/FILE000000CA.xxx.vmo".into(),
            bytes: 2,
            sha: [0x54; 32],
        },
        ArtifactFact {
            path: GAMEOBJECT_MODELS_DTREE_PATH.into(),
            bytes: 8 + 2 * (8 + 16 + 24),
            sha: [0x52; 32],
        },
    ]);
    coverage
}

fn gameobject_missing_root_material_filtered_wmo_sample() -> Coverage {
    let mut coverage = gameobject_expanded_material_filtered_wmo_sample();
    coverage.resource_profile = resources::Profile::GameObjectMissingRootExpandedV1;
    coverage.gameobject_scope = geometry::GameObjectScope::AvailableFdidDirectMissingRootV1;
    coverage.model_scope = ModelScope::PlacementAndGameObjectMissingRoot;
    coverage.gameobject_missing_root = vec![201, 202];
    let facts = coverage.gameobjects.as_mut().unwrap();
    facts.missing_root_rows = 2;
    facts.missing_root_fdids = vec![201, 202];
    facts.collision_rows = 0;
    facts.list_rows = 0;
    facts.list_m2_rows = 0;
    facts.list_wmo_rows = 0;
    facts.list_vertex_visits = 0;
    facts.list_raw_visit_bytes = 0;
    facts.temp_bytes = forever_vmap::gameobject_models::TEMP_MAGIC.len() as u64;
    facts.dtree_bytes = forever_vmap::gameobject_models::DTREE_MAGIC.len() as u64;
    facts.temp_sha = integrity::digest(forever_vmap::gameobject_models::TEMP_MAGIC);
    facts.dtree_sha = integrity::digest(forever_vmap::gameobject_models::DTREE_MAGIC);
    facts.outcomes[2].disposition = GameObjectDisposition::MissingRoot;
    facts.outcomes[3].disposition = GameObjectDisposition::MissingRoot;
    coverage.artifacts.retain(|artifact| {
        !matches!(
            artifact.path.as_str(),
            "vmaps/FILE000000C9.xxx.vmo" | "vmaps/FILE000000CA.xxx.vmo"
        )
    });
    let tree = coverage
        .artifacts
        .iter_mut()
        .find(|artifact| artifact.path == GAMEOBJECT_MODELS_DTREE_PATH)
        .unwrap();
    tree.bytes = facts.dtree_bytes;
    tree.sha = facts.dtree_sha;
    coverage
}

fn gameobject_expanded_material_filtered_wmo_policy() -> GeometryPolicy {
    GeometryPolicy::new_with_material_filtered_wmo(
        MapSelection::All,
        run::MissingWdtPolicy::DeclareRootAbsent,
        run::MissingModelPolicy::DeclareRootAbsentWmoDoodadM2,
        resources::Profile::GameObjectExpandedV1,
    )
}

fn gameobject_missing_root_material_filtered_wmo_policy() -> GeometryPolicy {
    GeometryPolicy::new_with_material_filtered_wmo(
        MapSelection::All,
        run::MissingWdtPolicy::DeclareRootAbsent,
        run::MissingModelPolicy::DeclareRootAbsentWmoDoodadM2,
        resources::Profile::GameObjectMissingRootExpandedV1,
    )
    .with_gameobject_missing_root()
}

fn model_output_expanded_material_filtered_wmo_policy() -> GeometryPolicy {
    GeometryPolicy::new_with_material_filtered_wmo(
        MapSelection::All,
        run::MissingWdtPolicy::DeclareRootAbsent,
        run::MissingModelPolicy::DeclareRootAbsentWmoDoodadM2,
        resources::Profile::ModelOutputExpandedV1,
    )
}

fn model_bytes_expanded_material_filtered_wmo_policy() -> GeometryPolicy {
    GeometryPolicy::new_with_material_filtered_wmo(
        MapSelection::All,
        run::MissingWdtPolicy::DeclareRootAbsent,
        run::MissingModelPolicy::DeclareRootAbsentWmoDoodadM2,
        resources::Profile::ModelBytesExpandedV1,
    )
}

fn source_expanded_v2_material_filtered_wmo_policy() -> GeometryPolicy {
    GeometryPolicy::new_with_material_filtered_wmo(
        MapSelection::All,
        run::MissingWdtPolicy::DeclareRootAbsent,
        run::MissingModelPolicy::DeclareRootAbsentWmoDoodadM2,
        resources::Profile::SourceExpandedV2,
    )
}

fn placement_expanded_material_filtered_wmo_policy() -> GeometryPolicy {
    GeometryPolicy::new_with_material_filtered_wmo(
        MapSelection::All,
        run::MissingWdtPolicy::DeclareRootAbsent,
        run::MissingModelPolicy::DeclareRootAbsentWmoDoodadM2,
        resources::Profile::PlacementExpandedV1,
    )
}

fn assembly_expanded_material_filtered_wmo_policy() -> GeometryPolicy {
    GeometryPolicy::new_with_material_filtered_wmo(
        MapSelection::All,
        run::MissingWdtPolicy::DeclareRootAbsent,
        run::MissingModelPolicy::DeclareRootAbsentWmoDoodadM2,
        resources::Profile::AssemblyExpandedV1,
    )
}

fn source_expanded_material_filtered_wmo_policy() -> GeometryPolicy {
    GeometryPolicy::new_with_material_filtered_wmo(
        MapSelection::All,
        run::MissingWdtPolicy::DeclareRootAbsent,
        run::MissingModelPolicy::DeclareRootAbsentWmoDoodadM2,
        resources::Profile::SourceExpandedV1,
    )
}

fn model_heavy_material_filtered_wmo_policy() -> GeometryPolicy {
    GeometryPolicy::new_with_model_heavy_material_filtered_wmo(
        MapSelection::All,
        run::MissingWdtPolicy::DeclareRootAbsent,
        run::MissingModelPolicy::DeclareRootAbsentWmoDoodadM2,
    )
}

fn read_heavy_material_filtered_wmo_policy() -> GeometryPolicy {
    GeometryPolicy::new_with_read_heavy_material_filtered_wmo(
        MapSelection::All,
        run::MissingWdtPolicy::DeclareRootAbsent,
        run::MissingModelPolicy::DeclareRootAbsentWmoDoodadM2,
    )
}

fn rich_sample() -> Coverage {
    let mut coverage = sample();
    coverage.map.declared_bytes = 4096;
    coverage.map.available_bytes = 1024;
    coverage.map.full_ckey_verified = false;
    coverage.map.declared_direct_rows = 7;
    coverage.map.declared_copy_records = 3;
    coverage.map.direct_rows = 5;
    coverage.map.copy_records = 2;
    coverage.map.materialized_rows = 7;
    coverage.map.sections = vec![
        SectionFact {
            ordinal: 0,
            records: 3,
            copies: 1,
            admitted: true,
        },
        SectionFact {
            ordinal: 7,
            records: 2,
            copies: 1,
            admitted: true,
        },
        SectionFact {
            ordinal: 15,
            records: 2,
            copies: 1,
            admitted: false,
        },
    ];
    coverage.liquids[0].bytes = 12;
    coverage.liquids[0].direct_rows = 3;
    coverage.liquids[0].copy_records = 1;
    coverage.liquids[0].materialized_rows = 4;
    coverage.liquids[1].bytes = 16;
    coverage.liquids[1].direct_rows = 2;
    coverage.liquids[1].copy_records = 2;
    coverage.liquids[1].materialized_rows = 3;
    coverage.maps = vec![
        DomainMapFact {
            id: 1,
            parent: None,
            wdt_file_data_id: 99,
            depth: 0,
            is_parent: true,
            root_absent: false,
        },
        DomainMapFact {
            id: 2,
            parent: Some(1),
            wdt_file_data_id: 100,
            depth: 1,
            is_parent: false,
            root_absent: false,
        },
    ];
    coverage.artifacts.insert(
        0,
        ArtifactFact {
            path: "vmaps/0001/0001.vmtree".into(),
            bytes: 24,
            sha: [0x35; 32],
        },
    );
    coverage
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

fn mutate_text(bytes: &[u8], text: &str) -> Vec<u8> {
    let needle = text.as_bytes();
    let at = bytes
        .windows(needle.len())
        .position(|window| window == needle)
        .expect("coverage text");
    let mut mutated = bytes.to_vec();
    mutated[at] ^= 1;
    mutated
}

fn replace_text(bytes: &[u8], text: &str, replacement: &str) -> Vec<u8> {
    let needle = text.as_bytes();
    let at = bytes
        .windows(needle.len())
        .position(|window| window == needle)
        .expect("coverage text");
    let mut mutated = bytes[..at - 4].to_vec();
    mutated.extend_from_slice(&(replacement.len() as u32).to_le_bytes());
    mutated.extend_from_slice(replacement.as_bytes());
    mutated.extend_from_slice(&bytes[at + needle.len()..]);
    mutated
}

fn after_context(bytes: &[u8]) -> usize {
    let length = u32::from_le_bytes(bytes[12..16].try_into().unwrap()) as usize;
    16 + length
}

const SELECTED_V1_GOLDEN_HEX: &str = concat!(
    "52435647454f310001000000280100007b226275696c64223a37303435302c226c6f63616c65223a22656e5553222c22",
    "636f6d706f6e656e7473223a31362c22736f757263655f736861323536223a2233313331333133313331333133313331",
    "333133313331333133313331333133313331333133313331333133313331333133313331333133313331333133313331",
    "222c22706c616e5f736861323536223a2233323332333233323332333233323332333233323332333233323332333233",
    "323332333233323332333233323332333233323332333233323332333233323332222c2270726f64756365725f736861",
    "323536223a22333333333333333333333333333333333333333333333333333333333333333333333333333333333333",
    "33333333333333333333333333333333333333333333227d020000002c0000006c696e757837303435302d73656c6563",
    "7465642d6d61702d67656f6d657472792d636f7665726167652d7631230000006c696e757837303435302d73656c6563",
    "7465642d6d61702d67656f6d657472792d763121000000617661696c61626c652d6d61702d6163746976652d616e6365",
    "73746f72732d7631160000006578636c756465642d62792d70726f66696c652d763111000000706c6163656d656e742d",
    "6f6e6c792d763126000000617661696c61626c652d73656374696f6e2d6d61702d7061727469616c2d73636f70652d76",
    "3100080000006578636c75646564130000006578636c756465645f62795f70726f66696c65000e000000706c6163656d",
    "656e745f6f6e6c7925000000636f6d706c6574652d4c69717569644d6174657269616c2d4c6971756964547970652d76",
    "310c0000006f75745f6f665f73636f70650000655213000000000000000000ff03000065971400cc00000000000000cc",
    "000000000000000101000000000000000000000000000000000000000000000000000000000000000000000000000000",
    "00010000000000000000000000000000000102000000fa47110004000000000000004d41544c00000000000000000000",
    "0000000000000000000000000000f4ec1400040000000000000054595045000000000000000000000000000000000000",
    "000000000000010000000200000000000000006400000000000000000100000016000000766d6170732f303030322f30",
    "3030322e766d747265651000000000000000343434343434343434343434343434343434343434343434343434343434",
    "3434",
);

const AVAILABLE_V2_GOLDEN_HEX: &str = concat!(
    "52435647454f310002000000280100007b226275696c64223a37303435302c226c6f63616c65223a22656e5553222c22636f",
    "6d706f6e656e7473223a31372c22736f757263655f736861323536223a223331333133313331333133313331333133313331",
    "3331333133313331333133313331333133313331333133313331333133313331333133313331333133313331222c22706c61",
    "6e5f736861323536223a22333233323332333233323332333233323332333233323332333233323332333233323332333233",
    "32333233323332333233323332333233323332333233323332222c2270726f64756365725f736861323536223a2233333333",
    "3333333333333333333333333333333333333333333333333333333333333333333333333333333333333333333333333333",
    "33333333333333333333227d2d0000006c696e757837303435302d617661696c61626c652d6d61702d67656f6d657472792d",
    "636f7665726167652d7632240000006c696e757837303435302d617661696c61626c652d6d61702d67656f6d657472792d76",
    "311b000000617661696c61626c652d6d61702d616c6c2d6163746976652d7631160000006578636c756465642d62792d7072",
    "6f66696c652d763111000000706c6163656d656e742d6f6e6c792d763126000000617661696c61626c652d73656374696f6e",
    "2d6d61702d7061727469616c2d73636f70652d763100080000006578636c75646564130000006578636c756465645f62795f",
    "70726f66696c65000e000000706c6163656d656e745f6f6e6c7925000000636f6d706c6574652d4c69717569644d61746572",
    "69616c2d4c6971756964547970652d76310c0000006f75745f6f665f73636f70650000655213000000000000000000ff0300",
    "0065971400cc00000000000000cc000000000000000101000000000000000000000000000000000000000000000000000000",
    "00000000000000000000000000010000000000000000000000000000000102000000fa47110004000000000000004d41544c",
    "000000000000000000000000000000000000000000000000f4ec140004000000000000005459504500000000000000000000",
    "0000000000000000000000000000010000000200000000000000006400000000000000000100000016000000766d6170732f",
    "303030322f303030322e766d7472656510000000000000003434343434343434343434343434343434343434343434343434",
    "343434343434",
);

const ROOT_ABSENT_V3_GOLDEN_HEX: &str = concat!(
    "52435647454f310003000000280100007b226275696c64223a37303435302c226c6f63616c65223a22656e5553222c22636f",
    "6d706f6e656e7473223a31372c22736f757263655f736861323536223a223331333133313331333133313331333133313331",
    "3331333133313331333133313331333133313331333133313331333133313331333133313331333133313331222c22706c61",
    "6e5f736861323536223a22333233323332333233323332333233323332333233323332333233323332333233323332333233",
    "32333233323332333233323332333233323332333233323332222c2270726f64756365725f736861323536223a2233333333",
    "3333333333333333333333333333333333333333333333333333333333333333333333333333333333333333333333333333",
    "33333333333333333333227d330000006c696e757837303435302d617661696c61626c652d6d61702d726f6f742d6f6d6973",
    "73696f6e732d636f7665726167652d76332a0000006c696e757837303435302d617661696c61626c652d6d61702d726f6f74",
    "2d6f6d697373696f6e732d76311f000000617661696c61626c652d6d61702d726f6f742d6f6d697373696f6e732d76310116",
    "0000006578636c756465642d62792d70726f66696c652d763111000000706c6163656d656e742d6f6e6c792d763126000000",
    "617661696c61626c652d73656374696f6e2d6d61702d7061727469616c2d73636f70652d763100080000006578636c756465",
    "64130000006578636c756465645f62795f70726f66696c65000e000000706c6163656d656e745f6f6e6c7925000000636f6d",
    "706c6574652d4c69717569644d6174657269616c2d4c6971756964547970652d76310c0000006f75745f6f665f73636f7065",
    "0000655213000000000000000000ff03000065971400cc00000000000000cc00000000000000010100000000000000000000",
    "0000000000000000000000000000000000000000000000000000000000000100000000000000000000000000000001020000",
    "00fa47110004000000000000004d41544c000000000000000000000000000000000000000000000000f4ec14000400000000",
    "0000005459504500000000000000000000000000000000000000000000000004000000010000000000000000630000000000",
    "0000010002000000010100000064000000010000000101030000000102000000650000000200000000000500000001020000",
    "006500000002000000000100000000",
);

const MODEL_ROOT_ABSENT_V4_GOLDEN_HEX: &str = concat!(
    "52435647454f310004000000280100007b226275696c64223a37303435302c226c6f63616c65223a22656e5553222c22636f",
    "6d706f6e656e7473223a31372c22736f757263655f736861323536223a223331333133313331333133313331333133313331",
    "3331333133313331333133313331333133313331333133313331333133313331333133313331333133313331222c22706c61",
    "6e5f736861323536223a22333233323332333233323332333233323332333233323332333233323332333233323332333233",
    "32333233323332333233323332333233323332333233323332222c2270726f64756365725f736861323536223a2233333333",
    "3333333333333333333333333333333333333333333333333333333333333333333333333333333333333333333333333333",
    "33333333333333333333227d390000006c696e757837303435302d617661696c61626c652d6d61702d6d6f64656c2d726f6f",
    "742d6f6d697373696f6e732d636f7665726167652d7634300000006c696e757837303435302d617661696c61626c652d6d61",
    "702d6d6f64656c2d726f6f742d6f6d697373696f6e732d763125000000617661696c61626c652d6d61702d6d6f64656c2d72",
    "6f6f742d6f6d697373696f6e732d763101160000006578636c756465642d62792d70726f66696c652d763111000000706c61",
    "63656d656e742d6f6e6c792d763126000000617661696c61626c652d73656374696f6e2d6d61702d7061727469616c2d7363",
    "6f70652d763100080000006578636c75646564130000006578636c756465645f62795f70726f66696c65000e000000706c61",
    "63656d656e745f6f6e6c7925000000636f6d706c6574652d4c69717569644d6174657269616c2d4c6971756964547970652d",
    "76310c0000006f75745f6f665f73636f70650000655213000000000000000000ff03000065971400cc00000000000000cc00",
    "0000000000000101000000000000000000000000000000000000000000000000000000000000000000000000000000000100",
    "00000000000000000000000000000102000000fa47110004000000000000004d41544c000000000000000000000000000000",
    "000000000000000000f4ec140004000000000000005459504500000000000000000000000000000000000000000000000004",
    "0000000100000000000000006300000000000000010002000000010100000064000000010000000101030000000102000000",
    "65000000020000000000050000000102000000650000000200000000010102000000443322118877665500000000",
);

const REQUEST_CAPACITY_V5_GOLDEN_HEX: &str = concat!(
    "52435647454f310005000000280100007b226275696c64223a37303435302c226c6f63616c65223a22656e5553222c22636f",
    "6d706f6e656e7473223a31372c22736f757263655f736861323536223a223331333133313331333133313331333133313331",
    "3331333133313331333133313331333133313331333133313331333133313331333133313331333133313331222c22706c61",
    "6e5f736861323536223a22333233323332333233323332333233323332333233323332333233323332333233323332333233",
    "32333233323332333233323332333233323332333233323332222c2270726f64756365725f736861323536223a2233333333",
    "3333333333333333333333333333333333333333333333333333333333333333333333333333333333333333333333333333",
    "33333333333333333333227d4a0000006c696e757837303435302d617661696c61626c652d6d61702d6d6f64656c2d726f6f",
    "742d6f6d697373696f6e732d726571756573742d63617061636974792d636f7665726167652d7635410000006c696e757837",
    "303435302d617661696c61626c652d6d61702d6d6f64656c2d726f6f742d6f6d697373696f6e732d726571756573742d6361",
    "7061636974792d763136000000617661696c61626c652d6d61702d6d6f64656c2d726f6f742d6f6d697373696f6e732d7265",
    "71756573742d63617061636974792d763101160000006578636c756465642d62792d70726f66696c652d763111000000706c",
    "6163656d656e742d6f6e6c792d763126000000617661696c61626c652d73656374696f6e2d6d61702d7061727469616c2d73",
    "636f70652d763100080000006578636c75646564130000006578636c756465645f62795f70726f66696c65000e000000706c",
    "6163656d656e745f6f6e6c7925000000636f6d706c6574652d4c69717569644d6174657269616c2d4c697175696454797065",
    "2d76310c0000006f75745f6f665f73636f70650000655213000000000000000000ff03000065971400cc00000000000000cc",
    "0000000000000001010000000000000000000000000000000000000000000000000000000000000000000000000000000001",
    "0000000000000000000000000000000102000000fa47110004000000000000004d41544c0000000000000000000000000000",
    "00000000000000000000f4ec1400040000000000000054595045000000000000000000000000000000000000000000000000",
    "0400000001000000000000000063000000000000000100020000000101000000640000000100000001010300000001020000",
    "006500000002000000000005000000010200000065000000020000000001010200000044332211887766552a0000006c696e",
    "75782d636f6c6c6973696f6e2d726571756573742d68656176792d7265736f75726365732d763100000000",
);

const VACATED_WMO_V6_GOLDEN_HEX: &str = concat!(
    "52435647454f310006000000280100007b226275696c64223a37303435302c226c6f63616c65223a22656e5553222c22636f",
    "6d706f6e656e7473223a31372c22736f757263655f736861323536223a223331333133313331333133313331333133313331",
    "3331333133313331333133313331333133313331333133313331333133313331333133313331333133313331222c22706c61",
    "6e5f736861323536223a22333233323332333233323332333233323332333233323332333233323332333233323332333233",
    "32333233323332333233323332333233323332333233323332222c2270726f64756365725f736861323536223a2233333333",
    "3333333333333333333333333333333333333333333333333333333333333333333333333333333333333333333333333333",
    "33333333333333333333227d2c0000006c696e757837303435302d617661696c61626c652d766163617465642d776d6f2d63",
    "6f7665726167652d7636230000006c696e757837303435302d617661696c61626c652d766163617465642d776d6f2d763118",
    "000000617661696c61626c652d766163617465642d776d6f2d763101160000006578636c756465642d62792d70726f66696c",
    "652d763111000000706c6163656d656e742d6f6e6c792d763126000000617661696c61626c652d73656374696f6e2d6d6170",
    "2d7061727469616c2d73636f70652d763100080000006578636c75646564130000006578636c756465645f62795f70726f66",
    "696c65000e000000706c6163656d656e745f6f6e6c7925000000636f6d706c6574652d4c69717569644d6174657269616c2d",
    "4c6971756964547970652d76310c0000006f75745f6f665f73636f70650000655213000000000000000000ff030000659714",
    "00cc00000000000000cc00000000000000010100000000000000000000000000000000000000000000000000000000000000",
    "000000000000000000010000000000000000000000000000000102000000fa47110004000000000000004d41544c00000000",
    "0000000000000000000000000000000000000000f4ec14000400000000000000545950450000000000000000000000000000",
    "0000000000000000000004000000010000000000000000630000000000000001000200000001010000006400000001000000",
    "0101030000000102000000650000000200000000000500000001020000006500000002000000000101020000004433221188",
    "7766552a0000006c696e75782d636f6c6c6973696f6e2d726571756573742d68656176792d7265736f75726365732d763101",
    "02000000ddccbbaa00ffeedd00000000",
);

const EMPTY_WMO_V7_GOLDEN_HEX: &str = concat!(
    "52435647454f310007000000280100007b226275696c64223a37303435302c226c6f63616c65223a22656e5553222c22636f",
    "6d706f6e656e7473223a31372c22736f757263655f736861323536223a223331333133313331333133313331333133313331",
    "3331333133313331333133313331333133313331333133313331333133313331333133313331333133313331222c22706c61",
    "6e5f736861323536223a22333233323332333233323332333233323332333233323332333233323332333233323332333233",
    "32333233323332333233323332333233323332333233323332222c2270726f64756365725f736861323536223a2233333333",
    "3333333333333333333333333333333333333333333333333333333333333333333333333333333333333333333333333333",
    "33333333333333333333227d2a0000006c696e757837303435302d617661696c61626c652d656d7074792d776d6f2d636f76",
    "65726167652d7637210000006c696e757837303435302d617661696c61626c652d656d7074792d776d6f2d76311600000061",
    "7661696c61626c652d656d7074792d776d6f2d763101160000006578636c756465642d62792d70726f66696c652d76311100",
    "0000706c6163656d656e742d6f6e6c792d763126000000617661696c61626c652d73656374696f6e2d6d61702d7061727469",
    "616c2d73636f70652d763100080000006578636c75646564130000006578636c756465645f62795f70726f66696c65000e00",
    "0000706c6163656d656e745f6f6e6c7925000000636f6d706c6574652d4c69717569644d6174657269616c2d4c6971756964",
    "547970652d76310c0000006f75745f6f665f73636f70650000655213000000000000000000ff03000065971400cc00000000",
    "000000cc00000000000000010100000000000000000000000000000000000000000000000000000000000000000000000000",
    "000000010000000000000000000000000000000102000000fa47110004000000000000004d41544c00000000000000000000",
    "0000000000000000000000000000f4ec14000400000000000000545950450000000000000000000000000000000000000000",
    "0000000004000000010000000000000000630000000000000001000200000001010000006400000001000000010103000000",
    "01020000006500000002000000000005000000010200000065000000020000000001010200000044332211887766552a0000",
    "006c696e75782d636f6c6c6973696f6e2d726571756573742d68656176792d7265736f75726365732d76310102000000ddcc",
    "bbaa00ffeedd02000000785634122143658700000000",
);

const DEPENDENCY_HEAVY_MATERIAL_FILTERED_WMO_V10_GOLDEN_HEX: &str = concat!(
    "52435647454f31000a000000280100007b226275696c6422",
    "3a37303435302c226c6f63616c65223a22656e5553222c22",
    "636f6d706f6e656e7473223a31372c22736f757263655f73",
    "6861323536223a2233313331333133313331333133313331",
    "333133313331333133313331333133313331333133313331",
    "333133313331333133313331333133313331333133313331",
    "222c22706c616e5f736861323536223a2233323332333233",
    "323332333233323332333233323332333233323332333233",
    "323332333233323332333233323332333233323332333233",
    "323332333233323332222c2270726f64756365725f736861",
    "323536223a22333333333333333333333333333333333333",
    "333333333333333333333333333333333333333333333333",
    "33333333333333333333333333333333333333333333227d",
    "3e0000006c696e757837303435302d646570656e64656e63",
    "792d68656176792d6d6174657269616c2d66696c74657265",
    "642d776d6f2d636f7665726167652d763130340000006c69",
    "6e757837303435302d646570656e64656e63792d68656176",
    "792d6d6174657269616c2d66696c74657265642d776d6f2d",
    "763129000000646570656e64656e63792d68656176792d6d",
    "6174657269616c2d66696c74657265642d776d6f2d763101",
    "160000006578636c756465642d62792d70726f66696c652d",
    "763111000000706c6163656d656e742d6f6e6c792d763126",
    "000000617661696c61626c652d73656374696f6e2d6d6170",
    "2d7061727469616c2d73636f70652d763100080000006578",
    "636c75646564130000006578636c756465645f62795f7072",
    "6f66696c65000e000000706c6163656d656e745f6f6e6c79",
    "25000000636f6d706c6574652d4c69717569644d61746572",
    "69616c2d4c6971756964547970652d76310c0000006f7574",
    "5f6f665f73636f70650000655213000000000000000000ff",
    "03000065971400cc00000000000000cc0000000000000001",
    "010000000000000000000000000000000000000000000000",
    "000000000000000000000000000000000001000000000000",
    "0000000000000000000102000000fa471100040000000000",
    "00004d41544c000000000000000000000000000000000000",
    "000000000000f4ec14000400000000000000545950450000",
    "000000000000000000000000000000000000000000000400",
    "000001000000000000000063000000000000000100020000",
    "000101000000640000000100000001010300000001020000",
    "006500000002000000000005000000010200000065000000",
    "02000000000101000000002d0000006c696e75782d636f6c",
    "6c6973696f6e2d646570656e64656e63792d68656176792d",
    "7265736f75726365732d7631010000000000000000000000",
    "000000000000000000",
);

const SOURCE_HEAVY_MATERIAL_FILTERED_WMO_V11_GOLDEN_HEX: &str = "52435647454f31000b000000280100007b226275696c64223a37303435302c226c6f63616c65223a22656e5553222c22636f6d706f6e656e7473223a31372c22736f757263655f736861323536223a2233313331333133313331333133313331333133313331333133313331333133313331333133313331333133313331333133313331333133313331333133313331222c22706c616e5f736861323536223a2233323332333233323332333233323332333233323332333233323332333233323332333233323332333233323332333233323332333233323332333233323332222c2270726f64756365725f736861323536223a2233333333333333333333333333333333333333333333333333333333333333333333333333333333333333333333333333333333333333333333333333333333227d3a0000006c696e757837303435302d736f757263652d68656176792d6d6174657269616c2d66696c74657265642d776d6f2d636f7665726167652d763131300000006c696e757837303435302d736f757263652d68656176792d6d6174657269616c2d66696c74657265642d776d6f2d763125000000736f757263652d68656176792d6d6174657269616c2d66696c74657265642d776d6f2d763101160000006578636c756465642d62792d70726f66696c652d763111000000706c6163656d656e742d6f6e6c792d763126000000617661696c61626c652d73656374696f6e2d6d61702d7061727469616c2d73636f70652d763100080000006578636c75646564130000006578636c756465645f62795f70726f66696c65000e000000706c6163656d656e745f6f6e6c7925000000636f6d706c6574652d4c69717569644d6174657269616c2d4c6971756964547970652d76310c0000006f75745f6f665f73636f70650000655213000000000000000000ff03000065971400cc00000000000000cc00000000000000010100000000000000000000000000000000000000000000000000000000000000000000000000000000010000000000000000000000000000000102000000fa47110004000000000000004d41544c000000000000000000000000000000000000000000000000f4ec140004000000000000005459504500000000000000000000000000000000000000000000000004000000010000000000000000630000000000000001000200000001010000006400000001000000010103000000010200000065000000020000000000050000000102000000650000000200000000010100000000290000006c696e75782d636f6c6c6973696f6e2d736f757263652d68656176792d7265736f75726365732d7631010000000000000000000000000000000000000000";

const READ_HEAVY_MATERIAL_FILTERED_WMO_V12_GOLDEN_HEX: &str = concat!(
    "52435647454f31000c000000280100007b226275696c64223a37303435302c226c6f63616c65223a22656e5553222c22",
    "636f6d706f6e656e7473223a31372c22736f757263655f736861323536223a2233313331333133313331333133313331",
    "333133313331333133313331333133313331333133313331333133313331333133313331333133313331333133313331",
    "222c22706c616e5f736861323536223a2233323332333233323332333233323332333233323332333233323332333233",
    "323332333233323332333233323332333233323332333233323332333233323332222c2270726f64756365725f736861",
    "323536223a22333333333333333333333333333333333333333333333333333333333333333333333333333333333333",
    "33333333333333333333333333333333333333333333227d380000006c696e757837303435302d726561642d68656176",
    "792d6d6174657269616c2d66696c74657265642d776d6f2d636f7665726167652d7631322e0000006c696e7578373034",
    "35302d726561642d68656176792d6d6174657269616c2d66696c74657265642d776d6f2d763123000000726561642d68",
    "656176792d6d6174657269616c2d66696c74657265642d776d6f2d763101160000006578636c756465642d62792d7072",
    "6f66696c652d763111000000706c6163656d656e742d6f6e6c792d763126000000617661696c61626c652d7365637469",
    "6f6e2d6d61702d7061727469616c2d73636f70652d763100080000006578636c75646564130000006578636c75646564",
    "5f62795f70726f66696c65000e000000706c6163656d656e745f6f6e6c7925000000636f6d706c6574652d4c69717569",
    "644d6174657269616c2d4c6971756964547970652d76310c0000006f75745f6f665f73636f7065000065521300000000",
    "0000000000ff03000065971400cc00000000000000cc0000000000000001010000000000000000000000000000000000",
    "0000000000000000000000000000000000000000000000010000000000000000000000000000000102000000fa471100",
    "04000000000000004d41544c000000000000000000000000000000000000000000000000f4ec14000400000000000000",
    "545950450000000000000000000000000000000000000000000000000400000001000000000000000063000000000000",
    "000100020000000101000000640000000100000001010300000001020000006500000002000000000005000000010200",
    "0000650000000200000000010100000000270000006c696e75782d636f6c6c6973696f6e2d726561642d68656176792d",
    "7265736f75726365732d7631010000000000000000000000000000000000000000",
);

const MODEL_BYTES_EXPANDED_MATERIAL_FILTERED_WMO_V16_GOLDEN_HEX: &str = concat!(
    "52435647454f310010000000280100007b226275696c64223a37303435302c226c6f63616c65223a22656e5553222c22636f",
    "6d706f6e656e7473223a31372c22736f757263655f736861323536223a223331333133313331333133313331",
    "3331333133313331333133313331333133313331333133313331333133313331333133313331333133313331333133313331222c22706c61",
    "6e5f736861323536223a22333233323332333233323332333233323332333233323332333233323332333233323332333233",
    "32333233323332333233323332333233323332333233323332222c2270726f64756365725f736861323536223a2233333333",
    "3333333333333333333333333333333333333333333333333333333333333333333333333333333333333333333333333333",
    "33333333333333333333227d420000006c696e757837303435302d6d6f64656c2d62797465732d657870616e6465642d6d61",
    "74657269616c2d66696c74657265642d776d6f2d636f7665726167652d763136380000006c696e757837303435302d6d6f64",
    "656c2d62797465732d657870616e6465642d6d6174657269616c2d66696c74657265642d776d6f2d76312d0000006d6f6465",
    "6c2d62797465732d657870616e6465642d6d6174657269616c2d66696c74657265642d776d6f2d763101160000006578636c",
    "756465642d62792d70726f66696c652d763111000000706c6163656d656e742d6f6e6c792d763126000000617661696c6162",
    "6c652d73656374696f6e2d6d61702d7061727469616c2d73636f70652d763100080000006578636c75646564130000006578",
    "636c756465645f62795f70726f66696c65000e000000706c6163656d656e745f6f6e6c7925000000636f6d706c6574652d4c",
    "69717569644d6174657269616c2d4c6971756964547970652d76310c0000006f75745f6f665f73636f706500006552130000",
    "00000000000000ff03000065971400cc00000000000000cc0000000000000001010000000000000000000000000000000000",
    "0000000000000000000000000000000000000000000000010000000000000000000000000000000102000000fa4711000400",
    "0000000000004d41544c000000000000000000000000000000000000000000000000f4ec1400040000000000000054595045",
    "0000000000000000000000000000000000000000000000000400000001000000000000000063000000000000000100020000",
    "0001010000006400000001000000010103000000010200000065000000020000000000050000000102000000650000000200",
    "000000010100000000310000006c696e75782d636f6c6c6973696f6e2d6d6f64656c2d62797465732d657870616e6465642d",
    "7265736f75726365732d7631010000000000000000000000000000000000000000",
);

#[test]
fn coverage_round_trip_retains_partial_map_scope_and_declared_policy_facts() {
    let coverage = sample();
    let bytes = coverage.encode().unwrap();
    let parsed = Coverage::parse(&bytes).unwrap();
    assert_eq!(parsed.context, coverage.context);
    assert_eq!(parsed.selection, MapSelection::MapAndAncestors(2));
    assert_eq!(parsed.gameobject_scope, coverage.gameobject_scope);
    assert_eq!(
        parsed.gameobject_file_data_id,
        coverage.gameobject_file_data_id
    );
    assert_eq!(
        parsed.gameobject_block_reason,
        coverage.gameobject_block_reason
    );
    assert_eq!(
        parsed.gameobject_request_made,
        coverage.gameobject_request_made
    );
    assert_eq!(
        parsed.gameobject_startup_reads,
        coverage.gameobject_startup_reads
    );
    assert_eq!(parsed.map, coverage.map);
    assert_eq!(parsed.model_scope, coverage.model_scope);
    assert_eq!(parsed.liquid_object_scope, coverage.liquid_object_scope);
    assert_eq!(
        parsed.liquid_object_request_made,
        coverage.liquid_object_request_made
    );
    assert_eq!(parsed.liquids, coverage.liquids);
    assert_eq!(parsed.maps, coverage.maps);
    assert_eq!(parsed.artifacts, coverage.artifacts);
    assert_eq!(bytes.get(..8), Some(COVERAGE_MAGIC.as_slice()));
}

#[test]
fn selected_coverage_v1_bytes_match_the_existing_golden() {
    let bytes = sample().encode().unwrap();
    assert_eq!(bytes.len(), 914);
    assert_eq!(hex(&bytes), SELECTED_V1_GOLDEN_HEX);
    assert!(Coverage::parse(&bytes).is_ok());
}

#[test]
fn coverage_v1_and_v2_round_trip_sections_liquids_maps_and_artifacts() {
    let mut available = rich_sample();
    available.selection = MapSelection::All;
    available.context.components = Component::Db2.bit() | Component::Collision.bit();
    for (coverage, parse) in [
        (
            rich_sample(),
            Coverage::parse as fn(&[u8]) -> Result<Coverage, Error>,
        ),
        (
            available,
            Coverage::parse_v29 as fn(&[u8]) -> Result<Coverage, Error>,
        ),
    ] {
        let bytes = coverage.encode().unwrap();
        let parsed = parse(&bytes).unwrap();
        assert_eq!(parsed.context, coverage.context);
        assert_eq!(parsed.selection, coverage.selection);
        assert_eq!(parsed.missing_wdt_policy, run::MissingWdtPolicy::Reject);
        assert_eq!(parsed.missing_m2_policy, run::MissingModelPolicy::Reject);
        assert_eq!(parsed.resource_profile, resources::Profile::BoundedV1);
        assert!(parsed.missing_m2.is_empty());
        assert_eq!(parsed.map, coverage.map);
        assert_eq!(parsed.map.sections, coverage.map.sections);
        assert_eq!(parsed.liquids, coverage.liquids);
        assert_eq!(parsed.maps, coverage.maps);
        assert_eq!(parsed.artifacts, coverage.artifacts);
    }
}

#[test]
fn coverage_v3_round_trip_keeps_policy_and_zero_or_multiple_root_omissions() {
    let mut without_omissions = root_absent_sample();
    for map in &mut without_omissions.maps {
        map.root_absent = false;
    }
    for coverage in [without_omissions, root_absent_sample()] {
        let bytes = coverage.encode().unwrap();
        assert_eq!(
            u32::from_le_bytes(bytes[8..12].try_into().unwrap()),
            ROOT_ABSENT_COVERAGE_VERSION
        );
        let parsed = Coverage::parse_v30(&bytes).unwrap();
        assert_eq!(parsed.context, coverage.context);
        assert_eq!(parsed.selection, MapSelection::All);
        assert_eq!(
            parsed.missing_wdt_policy,
            run::MissingWdtPolicy::DeclareRootAbsent
        );
        assert_eq!(parsed.missing_m2_policy, run::MissingModelPolicy::Reject);
        assert_eq!(parsed.resource_profile, resources::Profile::BoundedV1);
        assert!(parsed.missing_m2.is_empty());
        assert_eq!(parsed.maps, coverage.maps);
        assert_eq!(parsed.map, coverage.map);
        assert_eq!(parsed.artifacts, coverage.artifacts);
        assert!(Coverage::parse_v29(&bytes).is_err());
    }
}

#[test]
fn coverage_v4_round_trip_keeps_wmo_doodad_policy_and_ordered_missing_fdids() {
    let mut without_omissions = model_root_absent_sample();
    without_omissions.missing_m2.clear();
    for coverage in [without_omissions, model_root_absent_sample()] {
        let bytes = coverage.encode().unwrap();
        if !coverage.missing_m2.is_empty() {
            assert_eq!(hex(&bytes), MODEL_ROOT_ABSENT_V4_GOLDEN_HEX);
        }
        assert_eq!(
            u32::from_le_bytes(bytes[8..12].try_into().unwrap()),
            MODEL_ROOT_ABSENT_COVERAGE_VERSION
        );
        let parsed = Coverage::parse_v31(&bytes).unwrap();
        assert_eq!(parsed.context, coverage.context);
        assert_eq!(parsed.selection, MapSelection::All);
        assert_eq!(
            parsed.missing_wdt_policy,
            run::MissingWdtPolicy::DeclareRootAbsent
        );
        assert_eq!(
            parsed.missing_m2_policy,
            run::MissingModelPolicy::DeclareRootAbsentWmoDoodadM2
        );
        assert_eq!(parsed.resource_profile, resources::Profile::BoundedV1);
        assert_eq!(parsed.missing_m2, coverage.missing_m2);
        assert_eq!(parsed.maps, coverage.maps);
        assert_eq!(parsed.artifacts, coverage.artifacts);
        assert!(Coverage::parse_v30(&bytes).is_err());
    }
    assert_eq!(m2_file_name(0x1122_3344), "FILE11223344.xxx");
}

#[test]
fn coverage_v5_round_trip_keeps_request_heavy_resource_and_ordered_missing_fdids() {
    let mut without_omissions = request_capacity_sample();
    without_omissions.missing_m2.clear();
    for coverage in [without_omissions, request_capacity_sample()] {
        let bytes = coverage.encode().unwrap();
        if !coverage.missing_m2.is_empty() {
            assert_eq!(hex(&bytes), REQUEST_CAPACITY_V5_GOLDEN_HEX);
        }
        assert_eq!(
            u32::from_le_bytes(bytes[8..12].try_into().unwrap()),
            REQUEST_CAPACITY_COVERAGE_VERSION
        );
        let parsed = Coverage::parse_v32(&bytes).unwrap();
        assert_eq!(parsed.context, coverage.context);
        assert_eq!(parsed.selection, MapSelection::All);
        assert_eq!(
            parsed.missing_wdt_policy,
            run::MissingWdtPolicy::DeclareRootAbsent
        );
        assert_eq!(
            parsed.missing_m2_policy,
            run::MissingModelPolicy::DeclareRootAbsentWmoDoodadM2
        );
        assert_eq!(parsed.resource_profile, resources::Profile::RequestHeavyV1);
        assert_eq!(parsed.missing_m2, coverage.missing_m2);
        assert_eq!(parsed.maps, coverage.maps);
        assert_eq!(parsed.artifacts, coverage.artifacts);
        assert!(Coverage::parse_v31(&bytes).is_err());
    }
}

#[test]
fn coverage_v6_round_trip_keeps_reported_vacated_wmo_fdids() {
    let mut without_omissions = vacated_wmo_sample();
    without_omissions.missing_wmo.clear();
    for coverage in [without_omissions, vacated_wmo_sample()] {
        let bytes = coverage.encode().unwrap();
        if !coverage.missing_wmo.is_empty() {
            assert_eq!(hex(&bytes), VACATED_WMO_V6_GOLDEN_HEX);
        }
        assert_eq!(
            u32::from_le_bytes(bytes[8..12].try_into().unwrap()),
            VACATED_WMO_COVERAGE_VERSION
        );
        let parsed = Coverage::parse_v33(&bytes).unwrap();
        assert_eq!(parsed.selection, MapSelection::All);
        assert_eq!(parsed.missing_m2, coverage.missing_m2);
        assert_eq!(parsed.missing_wmo, coverage.missing_wmo);
        assert_eq!(
            parsed.missing_wmo_policy,
            run::MissingWmoPolicy::DeclareVacatedAfterRootAbsentM2
        );
        assert_eq!(parsed.resource_profile, resources::Profile::RequestHeavyV1);
        assert!(parsed.empty_wmo.is_empty());
        assert!(Coverage::parse_v32(&bytes).is_err());
    }
}

#[test]
fn coverage_v7_round_trip_keeps_disjoint_intrinsically_empty_wmo_ids() {
    let mut without_empty = empty_wmo_sample();
    without_empty.empty_wmo.clear();
    for coverage in [without_empty, empty_wmo_sample()] {
        let bytes = coverage.encode().unwrap();
        if !coverage.empty_wmo.is_empty() {
            assert_eq!(hex(&bytes), EMPTY_WMO_V7_GOLDEN_HEX);
        }
        assert_eq!(
            u32::from_le_bytes(bytes[8..12].try_into().unwrap()),
            EMPTY_WMO_COVERAGE_VERSION
        );
        let policy = GeometryPolicy::new_with_empty_wmo(
            coverage.selection,
            coverage.missing_wdt_policy,
            coverage.missing_m2_policy,
            coverage.resource_profile,
        );
        assert_eq!(coverage_version(policy), Ok(7));
        assert_eq!(coverage_path(policy).unwrap(), EMPTY_WMO_COVERAGE_PATH);
        assert_eq!(
            EMPTY_WMO_COVERAGE_PATH,
            "vmaps/available-empty-wmo-geometry-coverage-v7.bin"
        );
        assert_eq!(EMPTY_WMO_CODEC_PROFILE, "linux70450-available-empty-wmo-v1");
        assert_eq!(EMPTY_WMO_DOMAIN_PROFILE, "available-empty-wmo-v1");
        assert_eq!(
            EMPTY_WMO_COMPLETION_PROFILE,
            "linux70450-available-empty-wmo-collision-v1"
        );
        assert_eq!(
            crate::plan::EMPTY_WMO_COVERAGE_PROFILE,
            "linux70450-available-empty-wmo-coverage-v7"
        );
        let parsed = Coverage::parse_v34(&bytes).unwrap();
        assert_eq!(parsed.missing_wmo, coverage.missing_wmo);
        assert_eq!(parsed.empty_wmo, coverage.empty_wmo);
        assert!(parsed.unselected_wmo.is_empty());
        assert_eq!(
            parsed.missing_wmo_policy,
            run::MissingWmoPolicy::DeclareVacatedAfterRootAbsentM2OrEmpty
        );
        assert_eq!(parsed.missing_m2, coverage.missing_m2);
        assert_eq!(parsed.resource_profile, resources::Profile::RequestHeavyV1);
        assert!(Coverage::parse_v33(&bytes).is_err());
    }
}

#[test]
fn coverage_v7_accepts_each_omission_list_at_its_exact_limit() {
    let ids = |first: u32| (first..first + COVERAGE_MAX_EMPTY_WMO as u32).collect::<Vec<_>>();
    let full_coverage = || {
        let mut coverage = empty_wmo_sample();
        coverage.missing_m2 = ids(1);
        coverage.missing_wmo = ids(4097);
        coverage.empty_wmo = ids(8193);
        coverage
    };

    let bytes = full_coverage().encode().unwrap();
    let parsed = Coverage::parse_v34(&bytes).unwrap();
    assert_eq!(parsed.missing_m2.len(), COVERAGE_MAX_MISSING_M2);
    assert_eq!(parsed.missing_wmo.len(), COVERAGE_MAX_MISSING_WMO);
    assert_eq!(parsed.empty_wmo.len(), COVERAGE_MAX_EMPTY_WMO);

    for field in [0, 1, 2] {
        let mut overfull = full_coverage();
        match field {
            0 => overfull.missing_m2.push(20481),
            1 => overfull.missing_wmo.push(20481),
            _ => overfull.empty_wmo.push(20481),
        }
        assert_eq!(overfull.encode().unwrap_err().code, LIMIT);
    }
}

#[test]
fn coverage_v8_round_trip_tracks_unselected_wmo_and_preserves_v7_golden() {
    let mut without_unselected = unselected_wmo_sample();
    without_unselected.unselected_wmo.clear();
    for coverage in [without_unselected, unselected_wmo_sample()] {
        let bytes = coverage.encode().unwrap();
        assert_eq!(
            u32::from_le_bytes(bytes[8..12].try_into().unwrap()),
            UNSELECTED_WMO_COVERAGE_VERSION
        );
        assert_eq!(
            coverage_path(unselected_wmo_policy()).unwrap(),
            UNSELECTED_WMO_COVERAGE_PATH
        );
        let parsed = Coverage::parse_v35(&bytes).unwrap();
        assert_eq!(parsed.missing_m2, coverage.missing_m2);
        assert_eq!(parsed.missing_wmo, coverage.missing_wmo);
        assert_eq!(parsed.empty_wmo, coverage.empty_wmo);
        assert_eq!(parsed.unselected_wmo, coverage.unselected_wmo);
        assert_eq!(
            parsed.missing_wmo_policy,
            run::MissingWmoPolicy::DeclareVacatedAfterRootAbsentM2OrEmptyOrUnselected
        );
        assert_eq!(parsed.resource_profile, resources::Profile::RequestHeavyV1);
        assert!(Coverage::parse_v34(&bytes).is_err());
    }
    assert_eq!(
        coverage_version(unselected_wmo_policy()).unwrap(),
        UNSELECTED_WMO_COVERAGE_VERSION
    );

    let v7 = empty_wmo_sample().encode().unwrap();
    assert_eq!(hex(&v7), EMPTY_WMO_V7_GOLDEN_HEX);
    assert!(Coverage::parse_v34(&v7).is_ok());
}

#[test]
fn coverage_v9_round_trip_tracks_disjoint_material_filtered_wmo_ids() {
    let mut without_material_filtered = material_filtered_wmo_sample();
    without_material_filtered.material_filtered_wmo.clear();
    for coverage in [without_material_filtered, material_filtered_wmo_sample()] {
        let bytes = coverage.encode().unwrap();
        if !coverage.material_filtered_wmo.is_empty() {
            assert_eq!(
                hex(&integrity::digest(&bytes)),
                "15bf02d1f12618ae3d64fe80b4710e5f07639b608ef91453e97fcb1377ec48a0"
            );
        }
        assert_eq!(
            u32::from_le_bytes(bytes[8..12].try_into().unwrap()),
            MATERIAL_FILTERED_WMO_COVERAGE_VERSION
        );
        assert_eq!(
            coverage_path(material_filtered_wmo_policy()).unwrap(),
            MATERIAL_FILTERED_WMO_COVERAGE_PATH
        );
        let parsed = Coverage::parse_v36(&bytes).unwrap();
        assert_eq!(parsed.missing_m2, coverage.missing_m2);
        assert_eq!(parsed.missing_wmo, coverage.missing_wmo);
        assert_eq!(parsed.empty_wmo, coverage.empty_wmo);
        assert_eq!(parsed.unselected_wmo, coverage.unselected_wmo);
        assert_eq!(parsed.material_filtered_wmo, coverage.material_filtered_wmo);
        assert_eq!(
            parsed.missing_wmo_policy,
            run::MissingWmoPolicy::DeclareVacatedAfterRootAbsentM2OrEmptyOrUnselectedOrMaterialFiltered
        );
        assert_eq!(parsed.resource_profile, resources::Profile::RequestHeavyV1);
        assert!(Coverage::parse_v35(&bytes).is_err());
    }
    assert_eq!(
        coverage_version(material_filtered_wmo_policy()).unwrap(),
        MATERIAL_FILTERED_WMO_COVERAGE_VERSION
    );
    let v8 = unselected_wmo_sample().encode().unwrap();
    assert_eq!(
        u32::from_le_bytes(v8[8..12].try_into().unwrap()),
        UNSELECTED_WMO_COVERAGE_VERSION
    );
    assert!(Coverage::parse_v35(&v8).is_ok());
}

#[test]
fn coverage_v10_round_trip_tracks_dependency_heavy_material_filtered_wmo_ids() {
    let mut without_omissions = dependency_heavy_material_filtered_wmo_sample();
    without_omissions.missing_m2.clear();
    without_omissions.missing_wmo.clear();
    without_omissions.empty_wmo.clear();
    without_omissions.unselected_wmo.clear();
    without_omissions.material_filtered_wmo.clear();
    let mut with_omissions = dependency_heavy_material_filtered_wmo_sample();
    with_omissions.missing_m2 = vec![0x0102_0301, 0x0102_0302];
    with_omissions.missing_wmo = vec![0x0102_0303, 0x0102_0304];
    with_omissions.empty_wmo = vec![0x0102_0305, 0x0102_0306];
    with_omissions.unselected_wmo = vec![0x0102_0307, 0x0102_0308];
    with_omissions.material_filtered_wmo = vec![0x0102_0309, 0x0102_0310];
    for coverage in [without_omissions, with_omissions] {
        let bytes = coverage.encode().unwrap();
        if coverage.missing_m2.is_empty() {
            assert_eq!(
                hex(&bytes),
                DEPENDENCY_HEAVY_MATERIAL_FILTERED_WMO_V10_GOLDEN_HEX
            );
        }
        assert_eq!(
            u32::from_le_bytes(bytes[8..12].try_into().unwrap()),
            DEPENDENCY_HEAVY_MATERIAL_FILTERED_WMO_COVERAGE_VERSION
        );
        assert_eq!(
            coverage_path(dependency_heavy_material_filtered_wmo_policy()).unwrap(),
            DEPENDENCY_HEAVY_MATERIAL_FILTERED_WMO_COVERAGE_PATH
        );
        let parsed = Coverage::parse_v37(&bytes).unwrap();
        assert_eq!(parsed.missing_m2, coverage.missing_m2);
        assert_eq!(parsed.missing_wmo, coverage.missing_wmo);
        assert_eq!(parsed.empty_wmo, coverage.empty_wmo);
        assert_eq!(parsed.unselected_wmo, coverage.unselected_wmo);
        assert_eq!(parsed.material_filtered_wmo, coverage.material_filtered_wmo);
        assert_eq!(
            parsed.resource_profile,
            resources::Profile::DependencyHeavyV1
        );
        assert!(Coverage::parse_v36(&bytes).is_err());
    }
    assert_eq!(
        coverage_version(dependency_heavy_material_filtered_wmo_policy()).unwrap(),
        DEPENDENCY_HEAVY_MATERIAL_FILTERED_WMO_COVERAGE_VERSION
    );
    let v9 = material_filtered_wmo_sample().encode().unwrap();
    assert_eq!(
        u32::from_le_bytes(v9[8..12].try_into().unwrap()),
        MATERIAL_FILTERED_WMO_COVERAGE_VERSION
    );
    assert!(Coverage::parse_v36(&v9).is_ok());
}

#[test]
fn coverage_v11_round_trip_tracks_source_heavy_material_filtered_wmo_ids() {
    let mut without_omissions = source_heavy_material_filtered_wmo_sample();
    without_omissions.missing_m2.clear();
    without_omissions.missing_wmo.clear();
    without_omissions.empty_wmo.clear();
    without_omissions.unselected_wmo.clear();
    without_omissions.material_filtered_wmo.clear();
    let mut with_omissions = source_heavy_material_filtered_wmo_sample();
    with_omissions.missing_m2 = vec![0x0102_0301, 0x0102_0302];
    with_omissions.missing_wmo = vec![0x0102_0303, 0x0102_0304];
    with_omissions.empty_wmo = vec![0x0102_0305, 0x0102_0306];
    with_omissions.unselected_wmo = vec![0x0102_0307, 0x0102_0308];
    with_omissions.material_filtered_wmo = vec![0x0102_0309, 0x0102_0310];
    for coverage in [without_omissions, with_omissions] {
        let bytes = coverage.encode().unwrap();
        if coverage.missing_m2.is_empty() {
            assert_eq!(
                hex(&bytes),
                SOURCE_HEAVY_MATERIAL_FILTERED_WMO_V11_GOLDEN_HEX
            );
        }
        assert_eq!(
            u32::from_le_bytes(bytes[8..12].try_into().unwrap()),
            SOURCE_HEAVY_MATERIAL_FILTERED_WMO_COVERAGE_VERSION
        );
        assert_eq!(
            coverage_path(source_heavy_material_filtered_wmo_policy()).unwrap(),
            SOURCE_HEAVY_MATERIAL_FILTERED_WMO_COVERAGE_PATH
        );
        let parsed = Coverage::parse_v38(&bytes).unwrap();
        assert_eq!(parsed.selection, MapSelection::All);
        assert_eq!(
            parsed.context.components,
            Component::Db2.bit() | Component::Collision.bit()
        );
        assert_eq!(parsed.missing_m2, coverage.missing_m2);
        assert_eq!(parsed.missing_wmo, coverage.missing_wmo);
        assert_eq!(parsed.empty_wmo, coverage.empty_wmo);
        assert_eq!(parsed.unselected_wmo, coverage.unselected_wmo);
        assert_eq!(parsed.material_filtered_wmo, coverage.material_filtered_wmo);
        assert_eq!(parsed.resource_profile, resources::Profile::SourceHeavyV1);
        assert_eq!(parsed.maps, coverage.maps);
        assert_eq!(parsed.artifacts, coverage.artifacts);
        assert!(Coverage::parse_v37(&bytes).is_err());
        assert!(Coverage::parse_v36(&bytes).is_err());
    }
    assert_eq!(
        coverage_version(source_heavy_material_filtered_wmo_policy()).unwrap(),
        SOURCE_HEAVY_MATERIAL_FILTERED_WMO_COVERAGE_VERSION
    );
    assert_eq!(
        coverage_path(source_heavy_material_filtered_wmo_policy()).unwrap(),
        SOURCE_HEAVY_MATERIAL_FILTERED_WMO_COVERAGE_PATH
    );
    for (selection, missing_wdt, missing_m2, missing_wmo, resource, expected) in [
        (
            MapSelection::MapAndAncestors(2),
            run::MissingWdtPolicy::DeclareRootAbsent,
            run::MissingModelPolicy::DeclareRootAbsentWmoDoodadM2,
            run::MissingWmoPolicy::DeclareVacatedAfterRootAbsentM2OrEmptyOrUnselectedOrMaterialFiltered,
            resources::Profile::SourceHeavyV1,
            None,
        ),
        (
            MapSelection::All,
            run::MissingWdtPolicy::Reject,
            run::MissingModelPolicy::DeclareRootAbsentWmoDoodadM2,
            run::MissingWmoPolicy::DeclareVacatedAfterRootAbsentM2OrEmptyOrUnselectedOrMaterialFiltered,
            resources::Profile::SourceHeavyV1,
            None,
        ),
        (
            MapSelection::All,
            run::MissingWdtPolicy::DeclareRootAbsent,
            run::MissingModelPolicy::Reject,
            run::MissingWmoPolicy::DeclareVacatedAfterRootAbsentM2OrEmptyOrUnselectedOrMaterialFiltered,
            resources::Profile::SourceHeavyV1,
            None,
        ),
        (
            MapSelection::All,
            run::MissingWdtPolicy::DeclareRootAbsent,
            run::MissingModelPolicy::DeclareRootAbsentWmoDoodadM2,
            run::MissingWmoPolicy::Reject,
            resources::Profile::SourceHeavyV1,
            None,
        ),
        (
            MapSelection::All,
            run::MissingWdtPolicy::DeclareRootAbsent,
            run::MissingModelPolicy::DeclareRootAbsentWmoDoodadM2,
            run::MissingWmoPolicy::DeclareVacatedAfterRootAbsentM2OrEmptyOrUnselectedOrMaterialFiltered,
            resources::Profile::DependencyHeavyV1,
            Some(DEPENDENCY_HEAVY_MATERIAL_FILTERED_WMO_COVERAGE_VERSION),
        ),
        (
            MapSelection::All,
            run::MissingWdtPolicy::DeclareRootAbsent,
            run::MissingModelPolicy::DeclareRootAbsentWmoDoodadM2,
            run::MissingWmoPolicy::DeclareVacatedAfterRootAbsentM2OrEmptyOrUnselectedOrMaterialFiltered,
            resources::Profile::RequestHeavyV1,
            Some(MATERIAL_FILTERED_WMO_COVERAGE_VERSION),
        ),
    ] {
        let policy = GeometryPolicy::new_with_missing_wmo(
            selection,
            missing_wdt,
            missing_m2,
            missing_wmo,
            resource,
        );
        if let Some(expected) = expected {
            assert_eq!(coverage_version(policy).unwrap(), expected);
        } else {
            assert!(coverage_version(policy).is_err());
            assert!(coverage_path(policy).is_err());
        }
    }
}

#[test]
fn coverage_v12_round_trip_tracks_read_heavy_material_filtered_wmo_ids() {
    let mut without_omissions = read_heavy_material_filtered_wmo_sample();
    without_omissions.missing_m2.clear();
    without_omissions.missing_wmo.clear();
    without_omissions.empty_wmo.clear();
    without_omissions.unselected_wmo.clear();
    without_omissions.material_filtered_wmo.clear();
    let mut with_omissions = read_heavy_material_filtered_wmo_sample();
    with_omissions.missing_m2 = vec![0x0102_0301, 0x0102_0302];
    with_omissions.missing_wmo = vec![0x0102_0303, 0x0102_0304];
    with_omissions.empty_wmo = vec![0x0102_0305, 0x0102_0306];
    with_omissions.unselected_wmo = vec![0x0102_0307, 0x0102_0308];
    with_omissions.material_filtered_wmo = vec![0x0102_0309, 0x0102_0310];
    for coverage in [without_omissions, with_omissions] {
        let bytes = coverage.encode().unwrap();
        if coverage.missing_m2.is_empty() {
            assert_eq!(hex(&bytes), READ_HEAVY_MATERIAL_FILTERED_WMO_V12_GOLDEN_HEX);
            assert_eq!(
                hex(&integrity::digest(&bytes)),
                "1b21c4a5351ab658b543af172d5a6c30997948b099ba47c8aafb866cde46bf72"
            );
        }
        assert_eq!(
            u32::from_le_bytes(bytes[8..12].try_into().unwrap()),
            READ_HEAVY_MATERIAL_FILTERED_WMO_COVERAGE_VERSION
        );
        assert_eq!(
            coverage_path(read_heavy_material_filtered_wmo_policy()).unwrap(),
            READ_HEAVY_MATERIAL_FILTERED_WMO_COVERAGE_PATH
        );
        let parsed = Coverage::parse_v39(&bytes).unwrap();
        assert_eq!(parsed.selection, MapSelection::All);
        assert_eq!(
            parsed.context.components,
            Component::Db2.bit() | Component::Collision.bit()
        );
        assert_eq!(parsed.missing_m2, coverage.missing_m2);
        assert_eq!(parsed.missing_wmo, coverage.missing_wmo);
        assert_eq!(parsed.empty_wmo, coverage.empty_wmo);
        assert_eq!(parsed.unselected_wmo, coverage.unselected_wmo);
        assert_eq!(parsed.material_filtered_wmo, coverage.material_filtered_wmo);
        assert_eq!(parsed.resource_profile, resources::Profile::ReadHeavyV1);
        assert_eq!(parsed.maps, coverage.maps);
        assert_eq!(parsed.artifacts, coverage.artifacts);
        assert!(Coverage::parse_v38(&bytes).is_err());
        assert!(Coverage::parse_v37(&bytes).is_err());
        assert!(Coverage::parse_v36(&bytes).is_err());
    }
    assert_eq!(
        coverage_version(read_heavy_material_filtered_wmo_policy()).unwrap(),
        READ_HEAVY_MATERIAL_FILTERED_WMO_COVERAGE_VERSION
    );
    assert_eq!(
        coverage_path(read_heavy_material_filtered_wmo_policy()).unwrap(),
        READ_HEAVY_MATERIAL_FILTERED_WMO_COVERAGE_PATH
    );
    for (selection, missing_wdt, missing_m2, missing_wmo, resource, expected) in [
        (
            MapSelection::MapAndAncestors(2),
            run::MissingWdtPolicy::DeclareRootAbsent,
            run::MissingModelPolicy::DeclareRootAbsentWmoDoodadM2,
            run::MissingWmoPolicy::DeclareVacatedAfterRootAbsentM2OrEmptyOrUnselectedOrMaterialFiltered,
            resources::Profile::ReadHeavyV1,
            None,
        ),
        (
            MapSelection::All,
            run::MissingWdtPolicy::Reject,
            run::MissingModelPolicy::DeclareRootAbsentWmoDoodadM2,
            run::MissingWmoPolicy::DeclareVacatedAfterRootAbsentM2OrEmptyOrUnselectedOrMaterialFiltered,
            resources::Profile::ReadHeavyV1,
            None,
        ),
        (
            MapSelection::All,
            run::MissingWdtPolicy::DeclareRootAbsent,
            run::MissingModelPolicy::Reject,
            run::MissingWmoPolicy::DeclareVacatedAfterRootAbsentM2OrEmptyOrUnselectedOrMaterialFiltered,
            resources::Profile::ReadHeavyV1,
            None,
        ),
        (
            MapSelection::All,
            run::MissingWdtPolicy::DeclareRootAbsent,
            run::MissingModelPolicy::DeclareRootAbsentWmoDoodadM2,
            run::MissingWmoPolicy::Reject,
            resources::Profile::ReadHeavyV1,
            None,
        ),
        (
            MapSelection::All,
            run::MissingWdtPolicy::DeclareRootAbsent,
            run::MissingModelPolicy::DeclareRootAbsentWmoDoodadM2,
            run::MissingWmoPolicy::DeclareVacatedAfterRootAbsentM2OrEmptyOrUnselectedOrMaterialFiltered,
            resources::Profile::SourceHeavyV1,
            Some(SOURCE_HEAVY_MATERIAL_FILTERED_WMO_COVERAGE_VERSION),
        ),
        (
            MapSelection::All,
            run::MissingWdtPolicy::DeclareRootAbsent,
            run::MissingModelPolicy::DeclareRootAbsentWmoDoodadM2,
            run::MissingWmoPolicy::DeclareVacatedAfterRootAbsentM2OrEmptyOrUnselectedOrMaterialFiltered,
            resources::Profile::DependencyHeavyV1,
            Some(DEPENDENCY_HEAVY_MATERIAL_FILTERED_WMO_COVERAGE_VERSION),
        ),
    ] {
        let policy = GeometryPolicy::new_with_missing_wmo(
            selection, missing_wdt, missing_m2, missing_wmo, resource,
        );
        if let Some(expected) = expected {
            assert_eq!(coverage_version(policy).unwrap(), expected);
        } else {
            assert!(coverage_version(policy).is_err());
            assert!(coverage_path(policy).is_err());
        }
    }
}

#[test]
fn coverage_v12_rejects_crossed_resources_and_noncanonical_material_filtered_lists() {
    let coverage = read_heavy_material_filtered_wmo_sample();
    let bytes = coverage.encode().unwrap();
    for resource in [
        resources::Profile::RequestHeavyV1,
        resources::Profile::DependencyHeavyV1,
        resources::Profile::SourceHeavyV1,
        resources::Profile::BoundedV1,
        resources::Profile::DiskWorldV1,
    ] {
        let crossed = replace_text(&bytes, resources::Profile::ReadHeavyV1.id(), resource.id());
        assert!(Coverage::parse_v39(&crossed).is_err());
    }

    let mut limited = read_heavy_material_filtered_wmo_sample();
    limited.material_filtered_wmo = (1..=COVERAGE_MAX_MATERIAL_FILTERED_WMO as u32 + 1).collect();
    assert_eq!(limited.encode().unwrap_err().code, LIMIT);
    for invalid in [vec![0], vec![9, 8], vec![9, 9]] {
        let mut invalid_ids = read_heavy_material_filtered_wmo_sample();
        invalid_ids.material_filtered_wmo = invalid;
        assert!(invalid_ids.encode().is_err());
    }
    assert_eq!(COVERAGE_MAX_MISSING_M2, 4096);
    assert_eq!(COVERAGE_MAX_MISSING_WMO, 4096);
    assert_eq!(COVERAGE_MAX_EMPTY_WMO, 4096);
    assert_eq!(COVERAGE_MAX_UNSELECTED_WMO, 4096);
    assert_eq!(COVERAGE_MAX_MATERIAL_FILTERED_WMO, 4096);
}

#[test]
fn coverage_v13_round_trip_tracks_model_heavy_material_filtered_wmo_ids() {
    let mut without_omissions = model_heavy_material_filtered_wmo_sample();
    without_omissions.missing_m2.clear();
    without_omissions.missing_wmo.clear();
    without_omissions.empty_wmo.clear();
    without_omissions.unselected_wmo.clear();
    without_omissions.material_filtered_wmo.clear();
    let mut with_omissions = model_heavy_material_filtered_wmo_sample();
    with_omissions.missing_m2 = vec![0x0102_0301, 0x0102_0302];
    with_omissions.missing_wmo = vec![0x0102_0303, 0x0102_0304];
    with_omissions.empty_wmo = vec![0x0102_0305, 0x0102_0306];
    with_omissions.unselected_wmo = vec![0x0102_0307, 0x0102_0308];
    with_omissions.material_filtered_wmo = vec![0x0102_0309, 0x0102_0310];
    for coverage in [without_omissions, with_omissions] {
        let bytes = coverage.encode().unwrap();
        if coverage.missing_m2.is_empty() {
            assert_eq!(
                hex(&integrity::digest(&bytes)),
                "d30eba3288ee52d247e1023c8fb2c5f3532fe9470f9e21573ef591351b13a55f"
            );
        }
        assert_eq!(
            u32::from_le_bytes(bytes[8..12].try_into().unwrap()),
            MODEL_HEAVY_MATERIAL_FILTERED_WMO_COVERAGE_VERSION
        );
        assert_eq!(
            coverage_path(model_heavy_material_filtered_wmo_policy()).unwrap(),
            MODEL_HEAVY_MATERIAL_FILTERED_WMO_COVERAGE_PATH
        );
        let parsed = Coverage::parse_v40(&bytes).unwrap();
        assert_eq!(parsed.selection, MapSelection::All);
        assert_eq!(
            parsed.context.components,
            Component::Db2.bit() | Component::Collision.bit()
        );
        assert_eq!(parsed.missing_m2, coverage.missing_m2);
        assert_eq!(parsed.missing_wmo, coverage.missing_wmo);
        assert_eq!(parsed.empty_wmo, coverage.empty_wmo);
        assert_eq!(parsed.unselected_wmo, coverage.unselected_wmo);
        assert_eq!(parsed.material_filtered_wmo, coverage.material_filtered_wmo);
        assert_eq!(parsed.resource_profile, resources::Profile::ModelHeavyV1);
        assert_eq!(parsed.maps, coverage.maps);
        assert_eq!(parsed.artifacts, coverage.artifacts);
        assert!(Coverage::parse_v39(&bytes).is_err());
        assert!(Coverage::parse_v38(&bytes).is_err());
        assert!(Coverage::parse_v37(&bytes).is_err());
        assert!(Coverage::parse_v36(&bytes).is_err());
    }
    assert_eq!(
        coverage_version(model_heavy_material_filtered_wmo_policy()).unwrap(),
        MODEL_HEAVY_MATERIAL_FILTERED_WMO_COVERAGE_VERSION
    );
    assert_eq!(
        coverage_path(model_heavy_material_filtered_wmo_policy()).unwrap(),
        MODEL_HEAVY_MATERIAL_FILTERED_WMO_COVERAGE_PATH
    );
    for (selection, missing_wdt, missing_m2, missing_wmo, resource, expected_version) in [
        (
            MapSelection::MapAndAncestors(2),
            run::MissingWdtPolicy::DeclareRootAbsent,
            run::MissingModelPolicy::DeclareRootAbsentWmoDoodadM2,
            run::MissingWmoPolicy::DeclareVacatedAfterRootAbsentM2OrEmptyOrUnselectedOrMaterialFiltered,
            resources::Profile::ModelHeavyV1,
            None,
        ),
        (
            MapSelection::All,
            run::MissingWdtPolicy::Reject,
            run::MissingModelPolicy::DeclareRootAbsentWmoDoodadM2,
            run::MissingWmoPolicy::DeclareVacatedAfterRootAbsentM2OrEmptyOrUnselectedOrMaterialFiltered,
            resources::Profile::ModelHeavyV1,
            None,
        ),
        (
            MapSelection::All,
            run::MissingWdtPolicy::DeclareRootAbsent,
            run::MissingModelPolicy::Reject,
            run::MissingWmoPolicy::DeclareVacatedAfterRootAbsentM2OrEmptyOrUnselectedOrMaterialFiltered,
            resources::Profile::ModelHeavyV1,
            None,
        ),
        (
            MapSelection::All,
            run::MissingWdtPolicy::DeclareRootAbsent,
            run::MissingModelPolicy::DeclareRootAbsentWmoDoodadM2,
            run::MissingWmoPolicy::Reject,
            resources::Profile::ModelHeavyV1,
            None,
        ),
        (
            MapSelection::All,
            run::MissingWdtPolicy::DeclareRootAbsent,
            run::MissingModelPolicy::DeclareRootAbsentWmoDoodadM2,
            run::MissingWmoPolicy::DeclareVacatedAfterRootAbsentM2OrEmptyOrUnselectedOrMaterialFiltered,
            resources::Profile::ReadHeavyV1,
            Some(READ_HEAVY_MATERIAL_FILTERED_WMO_COVERAGE_VERSION),
        ),
        (
            MapSelection::All,
            run::MissingWdtPolicy::DeclareRootAbsent,
            run::MissingModelPolicy::DeclareRootAbsentWmoDoodadM2,
            run::MissingWmoPolicy::DeclareVacatedAfterRootAbsentM2OrEmptyOrUnselectedOrMaterialFiltered,
            resources::Profile::SourceHeavyV1,
            Some(SOURCE_HEAVY_MATERIAL_FILTERED_WMO_COVERAGE_VERSION),
        ),
    ] {
        let policy = GeometryPolicy::new_with_missing_wmo(
            selection, missing_wdt, missing_m2, missing_wmo, resource,
        );
        if let Some(expected_version) = expected_version {
            assert_eq!(coverage_version(policy).unwrap(), expected_version);
            assert!(coverage_path(policy).is_ok());
        } else {
            assert!(coverage_version(policy).is_err());
            assert!(coverage_path(policy).is_err());
        }
    }
}

#[test]
fn coverage_v14_round_trip_tracks_source_expanded_material_filtered_wmo_ids() {
    let mut without_omissions = source_expanded_material_filtered_wmo_sample();
    without_omissions.missing_m2.clear();
    without_omissions.missing_wmo.clear();
    without_omissions.empty_wmo.clear();
    without_omissions.unselected_wmo.clear();
    without_omissions.material_filtered_wmo.clear();
    let mut with_omissions = source_expanded_material_filtered_wmo_sample();
    with_omissions.missing_m2 = vec![0x0102_0301, 0x0102_0302];
    with_omissions.missing_wmo = vec![0x0102_0303, 0x0102_0304];
    with_omissions.empty_wmo = vec![0x0102_0305, 0x0102_0306];
    with_omissions.unselected_wmo = vec![0x0102_0307, 0x0102_0308];
    with_omissions.material_filtered_wmo = vec![0x0102_0309, 0x0102_0310];
    for coverage in [without_omissions, with_omissions] {
        let bytes = coverage.encode().unwrap();
        if coverage.missing_m2.is_empty() {
            // Frozen v14 digest plus v13 normalization guards the retained field layout: only the
            // version and coverage, codec, domain, and resource identities may differ.
            assert_eq!(
                hex(&integrity::digest(&bytes)),
                "ad849167076501f54f59e71b80156247ea474eb6d38eaa545019ad70ed5fc486"
            );
            let mut model_heavy = model_heavy_material_filtered_wmo_sample();
            model_heavy.missing_m2.clear();
            model_heavy.missing_wmo.clear();
            model_heavy.empty_wmo.clear();
            model_heavy.unselected_wmo.clear();
            model_heavy.material_filtered_wmo.clear();
            assert_eq!(coverage.context, model_heavy.context);
            let mut normalized_v13 = bytes.clone();
            normalized_v13[8..12]
                .copy_from_slice(&MODEL_HEAVY_MATERIAL_FILTERED_WMO_COVERAGE_VERSION.to_le_bytes());
            for (source_expanded, model_heavy) in [
                (
                    crate::plan::SOURCE_EXPANDED_MATERIAL_FILTERED_WMO_COVERAGE_PROFILE,
                    crate::plan::MODEL_HEAVY_MATERIAL_FILTERED_WMO_COVERAGE_PROFILE,
                ),
                (
                    SOURCE_EXPANDED_MATERIAL_FILTERED_WMO_CODEC_PROFILE,
                    MODEL_HEAVY_MATERIAL_FILTERED_WMO_CODEC_PROFILE,
                ),
                (
                    SOURCE_EXPANDED_MATERIAL_FILTERED_WMO_DOMAIN_PROFILE,
                    MODEL_HEAVY_MATERIAL_FILTERED_WMO_DOMAIN_PROFILE,
                ),
                (
                    resources::Profile::SourceExpandedV1.id(),
                    resources::Profile::ModelHeavyV1.id(),
                ),
            ] {
                normalized_v13 = replace_text(&normalized_v13, source_expanded, model_heavy);
            }
            assert_eq!(normalized_v13, model_heavy.encode().unwrap());
        }
        assert_eq!(
            u32::from_le_bytes(bytes[8..12].try_into().unwrap()),
            SOURCE_EXPANDED_MATERIAL_FILTERED_WMO_COVERAGE_VERSION
        );
        assert_eq!(
            coverage_path(source_expanded_material_filtered_wmo_policy()).unwrap(),
            SOURCE_EXPANDED_MATERIAL_FILTERED_WMO_COVERAGE_PATH
        );
        let parsed = Coverage::parse_v41(&bytes).unwrap();
        assert_eq!(
            parsed.context.components,
            Component::Db2.bit() | Component::Collision.bit()
        );
        assert_eq!(parsed.missing_m2, coverage.missing_m2);
        assert_eq!(parsed.missing_wmo, coverage.missing_wmo);
        assert_eq!(parsed.empty_wmo, coverage.empty_wmo);
        assert_eq!(parsed.unselected_wmo, coverage.unselected_wmo);
        assert_eq!(parsed.material_filtered_wmo, coverage.material_filtered_wmo);
        assert_eq!(
            parsed.resource_profile,
            resources::Profile::SourceExpandedV1
        );
        assert_eq!(parsed.maps, coverage.maps);
        assert_eq!(parsed.artifacts, coverage.artifacts);
        assert!(Coverage::parse_v40(&bytes).is_err());
        assert!(Coverage::parse_v39(&bytes).is_err());
    }
    assert_eq!(
        coverage_version(source_expanded_material_filtered_wmo_policy()).unwrap(),
        SOURCE_EXPANDED_MATERIAL_FILTERED_WMO_COVERAGE_VERSION
    );
    assert_eq!(
        coverage_path(source_expanded_material_filtered_wmo_policy()).unwrap(),
        SOURCE_EXPANDED_MATERIAL_FILTERED_WMO_COVERAGE_PATH
    );
}

#[test]
fn coverage_v15_round_trip_tracks_model_output_expanded_material_filtered_wmo_ids() {
    let mut without_omissions = model_output_expanded_material_filtered_wmo_sample();
    without_omissions.missing_m2.clear();
    without_omissions.missing_wmo.clear();
    without_omissions.empty_wmo.clear();
    without_omissions.unselected_wmo.clear();
    without_omissions.material_filtered_wmo.clear();
    let mut with_omissions = model_output_expanded_material_filtered_wmo_sample();
    with_omissions.missing_m2 = vec![0x0102_0301, 0x0102_0302];
    with_omissions.missing_wmo = vec![0x0102_0303, 0x0102_0304];
    with_omissions.empty_wmo = vec![0x0102_0305, 0x0102_0306];
    with_omissions.unselected_wmo = vec![0x0102_0307, 0x0102_0308];
    with_omissions.material_filtered_wmo = vec![0x0102_0309, 0x0102_0310];
    for coverage in [without_omissions, with_omissions] {
        let bytes = coverage.encode().unwrap();
        if coverage.missing_m2.is_empty() {
            assert_eq!(
                hex(&integrity::digest(&bytes)),
                "433d78046326fdc9a2c40eee6b55c139e86ea844695bda1201546fadbba01b41"
            );
            let mut source_expanded = source_expanded_material_filtered_wmo_sample();
            source_expanded.missing_m2.clear();
            source_expanded.missing_wmo.clear();
            source_expanded.empty_wmo.clear();
            source_expanded.unselected_wmo.clear();
            source_expanded.material_filtered_wmo.clear();
            assert_eq!(coverage.context, source_expanded.context);
            let mut normalized_v14 = bytes.clone();
            normalized_v14[8..12].copy_from_slice(
                &SOURCE_EXPANDED_MATERIAL_FILTERED_WMO_COVERAGE_VERSION.to_le_bytes(),
            );
            for (model_output_expanded, source_expanded) in [
                (
                    crate::plan::MODEL_OUTPUT_EXPANDED_MATERIAL_FILTERED_WMO_COVERAGE_PROFILE,
                    crate::plan::SOURCE_EXPANDED_MATERIAL_FILTERED_WMO_COVERAGE_PROFILE,
                ),
                (
                    MODEL_OUTPUT_EXPANDED_MATERIAL_FILTERED_WMO_CODEC_PROFILE,
                    SOURCE_EXPANDED_MATERIAL_FILTERED_WMO_CODEC_PROFILE,
                ),
                (
                    MODEL_OUTPUT_EXPANDED_MATERIAL_FILTERED_WMO_DOMAIN_PROFILE,
                    SOURCE_EXPANDED_MATERIAL_FILTERED_WMO_DOMAIN_PROFILE,
                ),
                (
                    resources::Profile::ModelOutputExpandedV1.id(),
                    resources::Profile::SourceExpandedV1.id(),
                ),
            ] {
                normalized_v14 =
                    replace_text(&normalized_v14, model_output_expanded, source_expanded);
            }
            assert_eq!(normalized_v14, source_expanded.encode().unwrap());
        }
        assert_eq!(
            u32::from_le_bytes(bytes[8..12].try_into().unwrap()),
            MODEL_OUTPUT_EXPANDED_MATERIAL_FILTERED_WMO_COVERAGE_VERSION
        );
        assert_eq!(
            coverage_path(model_output_expanded_material_filtered_wmo_policy()).unwrap(),
            MODEL_OUTPUT_EXPANDED_MATERIAL_FILTERED_WMO_COVERAGE_PATH
        );
        let parsed = Coverage::parse_v42(&bytes).unwrap();
        assert_eq!(parsed.selection, MapSelection::All);
        assert_eq!(parsed.missing_m2, coverage.missing_m2);
        assert_eq!(parsed.missing_wmo, coverage.missing_wmo);
        assert_eq!(parsed.empty_wmo, coverage.empty_wmo);
        assert_eq!(parsed.unselected_wmo, coverage.unselected_wmo);
        assert_eq!(parsed.material_filtered_wmo, coverage.material_filtered_wmo);
        assert_eq!(
            parsed.resource_profile,
            resources::Profile::ModelOutputExpandedV1
        );
        assert_eq!(parsed.maps, coverage.maps);
        assert_eq!(parsed.artifacts, coverage.artifacts);
        assert!(Coverage::parse_v41(&bytes).is_err());
        assert!(Coverage::parse_v40(&bytes).is_err());
    }
    assert_eq!(
        coverage_version(model_output_expanded_material_filtered_wmo_policy()).unwrap(),
        MODEL_OUTPUT_EXPANDED_MATERIAL_FILTERED_WMO_COVERAGE_VERSION
    );
    assert_eq!(
        coverage_path(model_output_expanded_material_filtered_wmo_policy()).unwrap(),
        MODEL_OUTPUT_EXPANDED_MATERIAL_FILTERED_WMO_COVERAGE_PATH
    );
}

#[test]
fn coverage_v16_round_trip_tracks_model_bytes_expanded_material_filtered_wmo_ids() {
    let mut without_omissions = model_bytes_expanded_material_filtered_wmo_sample();
    without_omissions.missing_m2.clear();
    without_omissions.missing_wmo.clear();
    without_omissions.empty_wmo.clear();
    without_omissions.unselected_wmo.clear();
    without_omissions.material_filtered_wmo.clear();
    let mut with_omissions = model_bytes_expanded_material_filtered_wmo_sample();
    with_omissions.missing_m2 = vec![0x0102_0301, 0x0102_0302];
    with_omissions.missing_wmo = vec![0x0102_0303, 0x0102_0304];
    with_omissions.empty_wmo = vec![0x0102_0305, 0x0102_0306];
    with_omissions.unselected_wmo = vec![0x0102_0307, 0x0102_0308];
    with_omissions.material_filtered_wmo = vec![0x0102_0309, 0x0102_0310];
    for coverage in [without_omissions, with_omissions] {
        let bytes = coverage.encode().unwrap();
        if coverage.missing_m2.is_empty() {
            assert_eq!(bytes.len(), 1033);
            assert_eq!(
                hex(&bytes),
                MODEL_BYTES_EXPANDED_MATERIAL_FILTERED_WMO_V16_GOLDEN_HEX
            );
            assert_eq!(
                hex(&integrity::digest(&bytes)),
                "3b05c44c5226eee5c163e3af878f32c61e0eb49fb5aa069b75b3e480def70e8a"
            );
            let mut model_output_expanded = model_output_expanded_material_filtered_wmo_sample();
            model_output_expanded.missing_m2.clear();
            model_output_expanded.missing_wmo.clear();
            model_output_expanded.empty_wmo.clear();
            model_output_expanded.unselected_wmo.clear();
            model_output_expanded.material_filtered_wmo.clear();
            assert_eq!(coverage.context, model_output_expanded.context);
            let mut normalized_v15 = bytes.clone();
            normalized_v15[8..12].copy_from_slice(
                &MODEL_OUTPUT_EXPANDED_MATERIAL_FILTERED_WMO_COVERAGE_VERSION.to_le_bytes(),
            );
            for (model_bytes_expanded, model_output_expanded) in [
                (
                    crate::plan::MODEL_BYTES_EXPANDED_MATERIAL_FILTERED_WMO_COVERAGE_PROFILE,
                    crate::plan::MODEL_OUTPUT_EXPANDED_MATERIAL_FILTERED_WMO_COVERAGE_PROFILE,
                ),
                (
                    MODEL_BYTES_EXPANDED_MATERIAL_FILTERED_WMO_CODEC_PROFILE,
                    MODEL_OUTPUT_EXPANDED_MATERIAL_FILTERED_WMO_CODEC_PROFILE,
                ),
                (
                    MODEL_BYTES_EXPANDED_MATERIAL_FILTERED_WMO_DOMAIN_PROFILE,
                    MODEL_OUTPUT_EXPANDED_MATERIAL_FILTERED_WMO_DOMAIN_PROFILE,
                ),
                (
                    resources::Profile::ModelBytesExpandedV1.id(),
                    resources::Profile::ModelOutputExpandedV1.id(),
                ),
            ] {
                normalized_v15 =
                    replace_text(&normalized_v15, model_bytes_expanded, model_output_expanded);
            }
            assert_eq!(normalized_v15, model_output_expanded.encode().unwrap());
        }
        assert_eq!(
            u32::from_le_bytes(bytes[8..12].try_into().unwrap()),
            MODEL_BYTES_EXPANDED_MATERIAL_FILTERED_WMO_COVERAGE_VERSION
        );
        assert_eq!(
            coverage_path(model_bytes_expanded_material_filtered_wmo_policy()).unwrap(),
            MODEL_BYTES_EXPANDED_MATERIAL_FILTERED_WMO_COVERAGE_PATH
        );
        let parsed = Coverage::parse_v43(&bytes).unwrap();
        assert_eq!(parsed.selection, MapSelection::All);
        assert_eq!(parsed.missing_m2, coverage.missing_m2);
        assert_eq!(parsed.missing_wmo, coverage.missing_wmo);
        assert_eq!(parsed.empty_wmo, coverage.empty_wmo);
        assert_eq!(parsed.unselected_wmo, coverage.unselected_wmo);
        assert_eq!(parsed.material_filtered_wmo, coverage.material_filtered_wmo);
        assert_eq!(
            parsed.resource_profile,
            resources::Profile::ModelBytesExpandedV1
        );
        assert_eq!(parsed.maps, coverage.maps);
        assert_eq!(parsed.artifacts, coverage.artifacts);
        assert!(Coverage::parse_v42(&bytes).is_err());
        assert!(Coverage::parse_v41(&bytes).is_err());
    }
    assert_eq!(
        coverage_version(model_bytes_expanded_material_filtered_wmo_policy()).unwrap(),
        MODEL_BYTES_EXPANDED_MATERIAL_FILTERED_WMO_COVERAGE_VERSION
    );
    assert_eq!(
        coverage_path(model_bytes_expanded_material_filtered_wmo_policy()).unwrap(),
        MODEL_BYTES_EXPANDED_MATERIAL_FILTERED_WMO_COVERAGE_PATH
    );
}

#[test]
fn coverage_v17_round_trip_tracks_source_expanded_v2_material_filtered_wmo_ids() {
    let mut without_omissions = source_expanded_v2_material_filtered_wmo_sample();
    without_omissions.missing_m2.clear();
    without_omissions.missing_wmo.clear();
    without_omissions.empty_wmo.clear();
    without_omissions.unselected_wmo.clear();
    without_omissions.material_filtered_wmo.clear();
    let with_omissions = source_expanded_v2_material_filtered_wmo_sample();
    for coverage in [without_omissions, with_omissions] {
        let bytes = coverage.encode().unwrap();
        if coverage.missing_m2.is_empty() {
            assert_eq!(bytes.len(), 1022);
            assert_eq!(
                hex(&integrity::digest(&bytes)),
                "6ca62a384a97721adfaba63477df1b95921832d72571bbdc9298ccadbc60b418"
            );
        }
        assert_eq!(
            u32::from_le_bytes(bytes[8..12].try_into().unwrap()),
            SOURCE_EXPANDED_V2_MATERIAL_FILTERED_WMO_COVERAGE_VERSION
        );
        assert_eq!(
            coverage_path(source_expanded_v2_material_filtered_wmo_policy()).unwrap(),
            SOURCE_EXPANDED_V2_MATERIAL_FILTERED_WMO_COVERAGE_PATH
        );
        let parsed = Coverage::parse_v44(&bytes).unwrap();
        assert_eq!(parsed.context, coverage.context);
        assert_eq!(parsed.selection, coverage.selection);
        assert_eq!(parsed.missing_m2, coverage.missing_m2);
        assert_eq!(parsed.missing_wmo, coverage.missing_wmo);
        assert_eq!(parsed.empty_wmo, coverage.empty_wmo);
        assert_eq!(parsed.unselected_wmo, coverage.unselected_wmo);
        assert_eq!(parsed.material_filtered_wmo, coverage.material_filtered_wmo);
        assert_eq!(
            parsed.resource_profile,
            resources::Profile::SourceExpandedV2
        );
        assert_eq!(parsed.maps, coverage.maps);
        assert_eq!(parsed.artifacts, coverage.artifacts);
        assert_eq!(parsed.encode().unwrap(), bytes);
        assert!(Coverage::parse_v43(&bytes).is_err());
        assert!(Coverage::parse_v42(&bytes).is_err());

        let mut normalized_v16 = bytes.clone();
        normalized_v16[8..12].copy_from_slice(
            &MODEL_BYTES_EXPANDED_MATERIAL_FILTERED_WMO_COVERAGE_VERSION.to_le_bytes(),
        );
        for (source_expanded_v2, model_bytes_expanded) in [
            (
                crate::plan::SOURCE_EXPANDED_V2_MATERIAL_FILTERED_WMO_COVERAGE_PROFILE,
                crate::plan::MODEL_BYTES_EXPANDED_MATERIAL_FILTERED_WMO_COVERAGE_PROFILE,
            ),
            (
                SOURCE_EXPANDED_V2_MATERIAL_FILTERED_WMO_CODEC_PROFILE,
                MODEL_BYTES_EXPANDED_MATERIAL_FILTERED_WMO_CODEC_PROFILE,
            ),
            (
                SOURCE_EXPANDED_V2_MATERIAL_FILTERED_WMO_DOMAIN_PROFILE,
                MODEL_BYTES_EXPANDED_MATERIAL_FILTERED_WMO_DOMAIN_PROFILE,
            ),
            (
                resources::Profile::SourceExpandedV2.id(),
                resources::Profile::ModelBytesExpandedV1.id(),
            ),
        ] {
            normalized_v16 =
                replace_text(&normalized_v16, source_expanded_v2, model_bytes_expanded);
        }
        if coverage.missing_m2.is_empty() {
            assert_eq!(
                hex(&normalized_v16),
                MODEL_BYTES_EXPANDED_MATERIAL_FILTERED_WMO_V16_GOLDEN_HEX
            );
            assert_eq!(
                hex(&integrity::digest(&normalized_v16)),
                "3b05c44c5226eee5c163e3af878f32c61e0eb49fb5aa069b75b3e480def70e8a"
            );
        }
        let mut model_bytes_coverage = model_bytes_expanded_material_filtered_wmo_sample();
        model_bytes_coverage.missing_m2 = coverage.missing_m2.clone();
        model_bytes_coverage.missing_wmo = coverage.missing_wmo.clone();
        model_bytes_coverage.empty_wmo = coverage.empty_wmo.clone();
        model_bytes_coverage.unselected_wmo = coverage.unselected_wmo.clone();
        model_bytes_coverage.material_filtered_wmo = coverage.material_filtered_wmo.clone();
        assert_eq!(normalized_v16, model_bytes_coverage.encode().unwrap());
    }
    assert_eq!(
        coverage_version(source_expanded_v2_material_filtered_wmo_policy()).unwrap(),
        SOURCE_EXPANDED_V2_MATERIAL_FILTERED_WMO_COVERAGE_VERSION
    );
}

const PLACEMENT_EXPANDED_MATERIAL_FILTERED_WMO_V18_GOLDEN_BYTES: usize = 1025;
const PLACEMENT_EXPANDED_MATERIAL_FILTERED_WMO_V18_GOLDEN_SHA256: &str =
    "84414d20f4320e961a75622c2078d61dbc0315231409bb11afe3af9ed2d9b980";

#[test]
fn coverage_v18_round_trip_tracks_placement_expanded_material_filtered_wmo_ids() {
    let mut without_omissions = placement_expanded_material_filtered_wmo_sample();
    without_omissions.missing_m2.clear();
    without_omissions.missing_wmo.clear();
    without_omissions.empty_wmo.clear();
    without_omissions.unselected_wmo.clear();
    without_omissions.material_filtered_wmo.clear();
    let with_omissions = placement_expanded_material_filtered_wmo_sample();
    for coverage in [without_omissions, with_omissions] {
        let bytes = coverage.encode().unwrap();
        if coverage.missing_m2.is_empty() {
            assert_eq!(
                bytes.len(),
                PLACEMENT_EXPANDED_MATERIAL_FILTERED_WMO_V18_GOLDEN_BYTES
            );
            assert_eq!(
                hex(&integrity::digest(&bytes)),
                PLACEMENT_EXPANDED_MATERIAL_FILTERED_WMO_V18_GOLDEN_SHA256
            );
        }
        assert_eq!(
            u32::from_le_bytes(bytes[8..12].try_into().unwrap()),
            PLACEMENT_EXPANDED_MATERIAL_FILTERED_WMO_COVERAGE_VERSION
        );
        assert_eq!(
            coverage_path(placement_expanded_material_filtered_wmo_policy()).unwrap(),
            PLACEMENT_EXPANDED_MATERIAL_FILTERED_WMO_COVERAGE_PATH
        );
        let parsed = Coverage::parse_v45(&bytes).unwrap();
        assert_eq!(parsed.context, coverage.context);
        assert_eq!(parsed.selection, coverage.selection);
        assert_eq!(parsed.missing_m2, coverage.missing_m2);
        assert_eq!(parsed.missing_wmo, coverage.missing_wmo);
        assert_eq!(parsed.empty_wmo, coverage.empty_wmo);
        assert_eq!(parsed.unselected_wmo, coverage.unselected_wmo);
        assert_eq!(parsed.material_filtered_wmo, coverage.material_filtered_wmo);
        assert_eq!(
            parsed.resource_profile,
            resources::Profile::PlacementExpandedV1
        );
        assert_eq!(parsed.maps, coverage.maps);
        assert_eq!(parsed.artifacts, coverage.artifacts);
        assert_eq!(parsed.encode().unwrap(), bytes);
        assert!(Coverage::parse_v44(&bytes).is_err());
        assert!(Coverage::parse_v43(&bytes).is_err());

        let mut normalized_v17 = bytes.clone();
        normalized_v17[8..12].copy_from_slice(
            &SOURCE_EXPANDED_V2_MATERIAL_FILTERED_WMO_COVERAGE_VERSION.to_le_bytes(),
        );
        for (placement_expanded, source_expanded_v2) in [
            (
                crate::plan::PLACEMENT_EXPANDED_MATERIAL_FILTERED_WMO_COVERAGE_PROFILE,
                crate::plan::SOURCE_EXPANDED_V2_MATERIAL_FILTERED_WMO_COVERAGE_PROFILE,
            ),
            (
                PLACEMENT_EXPANDED_MATERIAL_FILTERED_WMO_CODEC_PROFILE,
                SOURCE_EXPANDED_V2_MATERIAL_FILTERED_WMO_CODEC_PROFILE,
            ),
            (
                PLACEMENT_EXPANDED_MATERIAL_FILTERED_WMO_DOMAIN_PROFILE,
                SOURCE_EXPANDED_V2_MATERIAL_FILTERED_WMO_DOMAIN_PROFILE,
            ),
            (
                resources::Profile::PlacementExpandedV1.id(),
                resources::Profile::SourceExpandedV2.id(),
            ),
        ] {
            normalized_v17 = replace_text(&normalized_v17, placement_expanded, source_expanded_v2);
        }
        let mut source_expanded_v2 = source_expanded_v2_material_filtered_wmo_sample();
        source_expanded_v2.missing_m2 = coverage.missing_m2.clone();
        source_expanded_v2.missing_wmo = coverage.missing_wmo.clone();
        source_expanded_v2.empty_wmo = coverage.empty_wmo.clone();
        source_expanded_v2.unselected_wmo = coverage.unselected_wmo.clone();
        source_expanded_v2.material_filtered_wmo = coverage.material_filtered_wmo.clone();
        let source_expanded_v2_bytes = source_expanded_v2.encode().unwrap();
        assert_eq!(normalized_v17, source_expanded_v2_bytes);
        if coverage.missing_m2.is_empty() {
            assert_eq!(normalized_v17.len(), 1022);
            assert_eq!(
                hex(&integrity::digest(&normalized_v17)),
                "6ca62a384a97721adfaba63477df1b95921832d72571bbdc9298ccadbc60b418"
            );
        }
    }
    assert_eq!(
        coverage_version(placement_expanded_material_filtered_wmo_policy()).unwrap(),
        PLACEMENT_EXPANDED_MATERIAL_FILTERED_WMO_COVERAGE_VERSION
    );
    assert_eq!(
        coverage_path(placement_expanded_material_filtered_wmo_policy()).unwrap(),
        PLACEMENT_EXPANDED_MATERIAL_FILTERED_WMO_COVERAGE_PATH
    );
}

const ASSEMBLY_EXPANDED_MATERIAL_FILTERED_WMO_V19_GOLDEN_BYTES: usize = 1021;
const ASSEMBLY_EXPANDED_MATERIAL_FILTERED_WMO_V19_GOLDEN_SHA256: &str =
    "f5e503bcf1f15c2682503ae8a6a7edd93ab600236b888006e9a941e8702e2c76";

#[test]
fn coverage_v19_round_trip_tracks_assembly_expanded_material_filtered_wmo_ids() {
    let mut without_omissions = assembly_expanded_material_filtered_wmo_sample();
    without_omissions.missing_m2.clear();
    without_omissions.missing_wmo.clear();
    without_omissions.empty_wmo.clear();
    without_omissions.unselected_wmo.clear();
    without_omissions.material_filtered_wmo.clear();
    let with_omissions = assembly_expanded_material_filtered_wmo_sample();
    for coverage in [without_omissions, with_omissions] {
        let bytes = coverage.encode().unwrap();
        if coverage.missing_m2.is_empty() {
            assert_eq!(
                bytes.len(),
                ASSEMBLY_EXPANDED_MATERIAL_FILTERED_WMO_V19_GOLDEN_BYTES
            );
            assert_eq!(
                hex(&integrity::digest(&bytes)),
                ASSEMBLY_EXPANDED_MATERIAL_FILTERED_WMO_V19_GOLDEN_SHA256
            );
        }
        assert_eq!(
            u32::from_le_bytes(bytes[8..12].try_into().unwrap()),
            ASSEMBLY_EXPANDED_MATERIAL_FILTERED_WMO_COVERAGE_VERSION
        );
        assert_eq!(
            coverage_path(assembly_expanded_material_filtered_wmo_policy()).unwrap(),
            ASSEMBLY_EXPANDED_MATERIAL_FILTERED_WMO_COVERAGE_PATH
        );
        let parsed = Coverage::parse_v46(&bytes).unwrap();
        assert_eq!(parsed.context, coverage.context);
        assert_eq!(parsed.selection, coverage.selection);
        assert_eq!(parsed.missing_m2, coverage.missing_m2);
        assert_eq!(parsed.missing_wmo, coverage.missing_wmo);
        assert_eq!(parsed.empty_wmo, coverage.empty_wmo);
        assert_eq!(parsed.unselected_wmo, coverage.unselected_wmo);
        assert_eq!(parsed.material_filtered_wmo, coverage.material_filtered_wmo);
        assert_eq!(
            parsed.resource_profile,
            resources::Profile::AssemblyExpandedV1
        );
        assert_eq!(parsed.maps, coverage.maps);
        assert_eq!(parsed.artifacts, coverage.artifacts);
        assert_eq!(parsed.encode().unwrap(), bytes);
        assert!(Coverage::parse_v45(&bytes).is_err());
        assert!(Coverage::parse_v44(&bytes).is_err());

        let mut normalized_v18 = bytes.clone();
        normalized_v18[8..12].copy_from_slice(
            &PLACEMENT_EXPANDED_MATERIAL_FILTERED_WMO_COVERAGE_VERSION.to_le_bytes(),
        );
        for (assembly_expanded, placement_expanded) in [
            (
                crate::plan::ASSEMBLY_EXPANDED_MATERIAL_FILTERED_WMO_COVERAGE_PROFILE,
                crate::plan::PLACEMENT_EXPANDED_MATERIAL_FILTERED_WMO_COVERAGE_PROFILE,
            ),
            (
                ASSEMBLY_EXPANDED_MATERIAL_FILTERED_WMO_CODEC_PROFILE,
                PLACEMENT_EXPANDED_MATERIAL_FILTERED_WMO_CODEC_PROFILE,
            ),
            (
                ASSEMBLY_EXPANDED_MATERIAL_FILTERED_WMO_DOMAIN_PROFILE,
                PLACEMENT_EXPANDED_MATERIAL_FILTERED_WMO_DOMAIN_PROFILE,
            ),
            (
                resources::Profile::AssemblyExpandedV1.id(),
                resources::Profile::PlacementExpandedV1.id(),
            ),
        ] {
            normalized_v18 = replace_text(&normalized_v18, assembly_expanded, placement_expanded);
        }
        let mut placement_expanded = placement_expanded_material_filtered_wmo_sample();
        placement_expanded.missing_m2 = coverage.missing_m2.clone();
        placement_expanded.missing_wmo = coverage.missing_wmo.clone();
        placement_expanded.empty_wmo = coverage.empty_wmo.clone();
        placement_expanded.unselected_wmo = coverage.unselected_wmo.clone();
        placement_expanded.material_filtered_wmo = coverage.material_filtered_wmo.clone();
        assert_eq!(normalized_v18, placement_expanded.encode().unwrap());
        assert!(Coverage::parse_v45(&normalized_v18).is_ok());
        assert!(Coverage::parse_v46(&normalized_v18).is_err());
    }
    assert_eq!(
        coverage_version(assembly_expanded_material_filtered_wmo_policy()).unwrap(),
        ASSEMBLY_EXPANDED_MATERIAL_FILTERED_WMO_COVERAGE_VERSION
    );
    assert_eq!(
        coverage_path(assembly_expanded_material_filtered_wmo_policy()).unwrap(),
        ASSEMBLY_EXPANDED_MATERIAL_FILTERED_WMO_COVERAGE_PATH
    );
}

#[test]
fn coverage_v19_rejects_crossed_resources_and_noncanonical_or_overlapping_lists() {
    let coverage = assembly_expanded_material_filtered_wmo_sample();
    let bytes = coverage.encode().unwrap();
    for resource in [
        resources::Profile::RequestHeavyV1,
        resources::Profile::DependencyHeavyV1,
        resources::Profile::SourceHeavyV1,
        resources::Profile::ReadHeavyV1,
        resources::Profile::ModelHeavyV1,
        resources::Profile::SourceExpandedV1,
        resources::Profile::ModelOutputExpandedV1,
        resources::Profile::ModelBytesExpandedV1,
        resources::Profile::SourceExpandedV2,
        resources::Profile::PlacementExpandedV1,
        resources::Profile::BoundedV1,
        resources::Profile::DiskWorldV1,
    ] {
        let crossed = replace_text(
            &bytes,
            resources::Profile::AssemblyExpandedV1.id(),
            resource.id(),
        );
        assert!(Coverage::parse_v46(&crossed).is_err());
    }
    for (identity, replacement) in [
        (
            ASSEMBLY_EXPANDED_MATERIAL_FILTERED_WMO_CODEC_PROFILE,
            PLACEMENT_EXPANDED_MATERIAL_FILTERED_WMO_CODEC_PROFILE,
        ),
        (
            ASSEMBLY_EXPANDED_MATERIAL_FILTERED_WMO_DOMAIN_PROFILE,
            PLACEMENT_EXPANDED_MATERIAL_FILTERED_WMO_DOMAIN_PROFILE,
        ),
        (
            crate::plan::ASSEMBLY_EXPANDED_MATERIAL_FILTERED_WMO_COVERAGE_PROFILE,
            crate::plan::PLACEMENT_EXPANDED_MATERIAL_FILTERED_WMO_COVERAGE_PROFILE,
        ),
    ] {
        assert!(Coverage::parse_v46(&replace_text(&bytes, identity, replacement)).is_err());
    }
    let mut crossed_version = bytes.clone();
    crossed_version[8..12]
        .copy_from_slice(&PLACEMENT_EXPANDED_MATERIAL_FILTERED_WMO_COVERAGE_VERSION.to_le_bytes());
    assert!(Coverage::parse_v46(&crossed_version).is_err());
    for invalid in [vec![0], vec![9, 8], vec![9, 9]] {
        let mut invalid_ids = assembly_expanded_material_filtered_wmo_sample();
        invalid_ids.material_filtered_wmo = invalid;
        assert!(invalid_ids.encode().is_err());
    }
    let mut overlap = assembly_expanded_material_filtered_wmo_sample();
    overlap.material_filtered_wmo[0] = overlap.missing_m2[1];
    assert!(overlap.encode().is_err());
}

#[test]
fn coverage_v20_round_trip_tracks_gameobject_outcomes_tree_and_model_artifacts() {
    const GOLDEN_BYTES: usize = 1696;
    const GOLDEN_SHA256: &str = "4bba9a414485adeb2e083a25b7969423e2ecccc72f9b2d2fcb0f25cc99fcb3b8";
    let mut without_omissions = gameobject_expanded_material_filtered_wmo_sample();
    without_omissions.missing_m2.clear();
    without_omissions.missing_wmo.clear();
    without_omissions.empty_wmo.clear();
    without_omissions.unselected_wmo.clear();
    without_omissions.material_filtered_wmo.clear();
    let with_omissions = gameobject_expanded_material_filtered_wmo_sample();

    for coverage in [without_omissions, with_omissions] {
        let bytes = coverage.encode().unwrap();
        if coverage.missing_m2.is_empty() {
            assert_eq!(bytes.len(), GOLDEN_BYTES);
            assert_eq!(hex(&integrity::digest(&bytes)), GOLDEN_SHA256);
        }
        assert_eq!(
            u32::from_le_bytes(bytes[8..12].try_into().unwrap()),
            GAMEOBJECT_EXPANDED_MATERIAL_FILTERED_WMO_COVERAGE_VERSION
        );
        assert_eq!(
            coverage_path(gameobject_expanded_material_filtered_wmo_policy()).unwrap(),
            GAMEOBJECT_EXPANDED_MATERIAL_FILTERED_WMO_COVERAGE_PATH
        );
        let parsed = Coverage::parse_v47(&bytes).unwrap();
        assert_eq!(parsed.context, coverage.context);
        assert_eq!(parsed.selection, coverage.selection);
        assert_eq!(parsed.missing_m2, coverage.missing_m2);
        assert_eq!(parsed.missing_wmo, coverage.missing_wmo);
        assert_eq!(parsed.empty_wmo, coverage.empty_wmo);
        assert_eq!(parsed.unselected_wmo, coverage.unselected_wmo);
        assert_eq!(parsed.material_filtered_wmo, coverage.material_filtered_wmo);
        assert_eq!(
            parsed.resource_profile,
            resources::Profile::GameObjectExpandedV1
        );
        assert_eq!(
            parsed.gameobject_scope,
            geometry::GameObjectScope::AvailableFdidDirectV1
        );
        assert_eq!(
            parsed.gameobject_block_reason,
            geometry::GameObjectBlockReason::None
        );
        assert!(parsed.gameobject_request_made);
        assert_eq!(parsed.gameobject_startup_reads, 1);
        assert_eq!(parsed.model_scope, ModelScope::PlacementAndGameObject);
        assert_eq!(parsed.gameobjects, coverage.gameobjects);
        assert_eq!(parsed.maps, coverage.maps);
        assert_eq!(parsed.artifacts, coverage.artifacts);
        assert!(
            parsed
                .artifacts
                .windows(2)
                .all(|pair| pair[0].path < pair[1].path)
        );
        assert_eq!(parsed.encode().unwrap(), bytes);
        assert!(Coverage::parse_v46(&bytes).is_err());
    }
    assert_eq!(
        coverage_version(gameobject_expanded_material_filtered_wmo_policy()).unwrap(),
        GAMEOBJECT_EXPANDED_MATERIAL_FILTERED_WMO_COVERAGE_VERSION
    );
    assert_eq!(
        coverage_path(gameobject_expanded_material_filtered_wmo_policy()).unwrap(),
        GAMEOBJECT_EXPANDED_MATERIAL_FILTERED_WMO_COVERAGE_PATH
    );
}

#[test]
fn gameobject_facts_allow_declared_copies_in_unadmitted_sections() {
    let mut coverage = gameobject_expanded_material_filtered_wmo_sample();
    let facts = coverage.gameobjects.as_mut().unwrap();
    facts.declared_copy_records = 3;
    facts.sections.push(GameObjectSectionFact {
        ordinal: 1,
        records: 0,
        copies: 2,
        admitted: false,
    });

    assert!(validate_gameobject_facts(facts, false, false).is_ok());
}

#[test]
fn gameobject_table_work_is_independent_of_candidate_requests() {
    let mut zero_fdids = gameobject_expanded_material_filtered_wmo_sample();
    let facts = zero_fdids.gameobjects.as_mut().unwrap();
    facts.candidate_requests = 0;
    facts.zero_fdids = facts.direct_outcomes;
    facts.textures = 0;
    facts.m2_no_collision = 0;
    facts.wmo_doodad_only = 0;
    facts.collision_rows = 0;
    facts.list_rows = 0;
    facts.list_m2_rows = 0;
    facts.list_wmo_rows = 0;
    facts.work_strict_candidates = 1;
    facts.work_partial_candidates = 0;
    for outcome in &mut facts.outcomes {
        outcome.file_data_id = 0;
        outcome.disposition = GameObjectDisposition::ZeroFileDataId;
    }
    assert!(validate_gameobject_facts(facts, false, false).is_ok());

    let mut distinct_work = gameobject_expanded_material_filtered_wmo_sample();
    let facts = distinct_work.gameobjects.as_mut().unwrap();
    facts.work_strict_candidates = 2;
    facts.work_partial_candidates = 5;
    assert!(validate_gameobject_facts(facts, false, false).is_ok());

    let mut work_overflow = gameobject_expanded_material_filtered_wmo_sample();
    let facts = work_overflow.gameobjects.as_mut().unwrap();
    facts.work_strict_candidates = u64::MAX;
    facts.work_partial_candidates = 1;
    assert!(validate_gameobject_facts(facts, false, false).is_err());
}

#[test]
fn coverage_v21_round_trip_tracks_missing_root_gameobjects_without_models() {
    let coverage = gameobject_missing_root_material_filtered_wmo_sample();
    let bytes = coverage.encode().unwrap();
    assert_eq!(
        u32::from_le_bytes(bytes[8..12].try_into().unwrap()),
        GAMEOBJECT_MISSING_ROOT_COVERAGE_VERSION
    );
    assert_eq!(
        coverage_version(gameobject_missing_root_material_filtered_wmo_policy()).unwrap(),
        GAMEOBJECT_MISSING_ROOT_COVERAGE_VERSION
    );
    assert_eq!(
        coverage_path(gameobject_missing_root_material_filtered_wmo_policy()).unwrap(),
        GAMEOBJECT_MISSING_ROOT_MATERIAL_FILTERED_WMO_COVERAGE_PATH
    );

    let parsed = Coverage::parse_v48(&bytes).unwrap();
    assert_eq!(parsed.context, coverage.context);
    assert_eq!(
        parsed.resource_profile,
        resources::Profile::GameObjectMissingRootExpandedV1
    );
    assert_eq!(
        parsed.gameobject_scope,
        geometry::GameObjectScope::AvailableFdidDirectMissingRootV1
    );
    assert_eq!(
        parsed.model_scope,
        ModelScope::PlacementAndGameObjectMissingRoot
    );
    assert_eq!(parsed.gameobject_missing_root, [201, 202]);
    let facts = parsed.gameobjects.as_ref().unwrap();
    assert_eq!(facts.missing_root_rows, 2);
    assert_eq!(facts.missing_root_fdids, [201, 202]);
    assert_eq!(
        facts.outcomes[2].disposition,
        GameObjectDisposition::MissingRoot
    );
    assert_eq!(
        facts.outcomes[3].disposition,
        GameObjectDisposition::MissingRoot
    );
    assert_eq!(parsed.encode().unwrap(), bytes);
    assert!(Coverage::parse_v47(&bytes).is_err());
    assert!(
        Coverage::parse_v48(
            &gameobject_expanded_material_filtered_wmo_sample()
                .encode()
                .unwrap()
        )
        .is_err()
    );

    for invalid in [vec![0], vec![201, 201], vec![202, 201]] {
        let mut malformed = gameobject_missing_root_material_filtered_wmo_sample();
        malformed.gameobject_missing_root = invalid.clone();
        malformed.gameobjects.as_mut().unwrap().missing_root_fdids = invalid;
        assert!(malformed.encode().is_err());
    }

    let mut mismatched = gameobject_missing_root_material_filtered_wmo_sample();
    mismatched.gameobject_missing_root = vec![202];
    mismatched.gameobjects.as_mut().unwrap().missing_root_fdids = vec![202];
    assert!(mismatched.encode().is_err());
}

#[test]
fn coverage_v20_rejects_crossed_identities_and_malformed_ordered_outcomes() {
    let coverage = gameobject_expanded_material_filtered_wmo_sample();
    let bytes = coverage.encode().unwrap();
    for (identity, replacement) in [
        (
            GAMEOBJECT_EXPANDED_MATERIAL_FILTERED_WMO_CODEC_PROFILE,
            ASSEMBLY_EXPANDED_MATERIAL_FILTERED_WMO_CODEC_PROFILE,
        ),
        (
            GAMEOBJECT_EXPANDED_MATERIAL_FILTERED_WMO_DOMAIN_PROFILE,
            ASSEMBLY_EXPANDED_MATERIAL_FILTERED_WMO_DOMAIN_PROFILE,
        ),
        (
            crate::plan::GAMEOBJECT_EXPANDED_MATERIAL_FILTERED_WMO_COVERAGE_PROFILE,
            crate::plan::ASSEMBLY_EXPANDED_MATERIAL_FILTERED_WMO_COVERAGE_PROFILE,
        ),
        (
            resources::Profile::GameObjectExpandedV1.id(),
            resources::Profile::AssemblyExpandedV1.id(),
        ),
    ] {
        assert!(Coverage::parse_v47(&replace_text(&bytes, identity, replacement)).is_err());
    }

    let mut reordered = gameobject_expanded_material_filtered_wmo_sample();
    reordered.gameobjects.as_mut().unwrap().outcomes.swap(0, 1);
    assert!(reordered.encode().is_err());

    let mut skipped_ordinal = gameobject_expanded_material_filtered_wmo_sample();
    skipped_ordinal.gameobjects.as_mut().unwrap().outcomes[2].ordinal = 3;
    assert!(skipped_ordinal.encode().is_err());

    let mut undersized_dtree = gameobject_expanded_material_filtered_wmo_sample();
    undersized_dtree.gameobjects.as_mut().unwrap().dtree_bytes =
        forever_vmap::gameobject_models::DTREE_MAGIC.len() as u64 - 1;
    assert!(undersized_dtree.encode().is_err());
}

#[test]
fn assembled_domain_budget_helpers_enforce_v45_v46_and_v47_boundaries() {
    use forever_casc::collision::resources::Profile;

    const MIB: u64 = 1024 * 1024;
    let legacy_limit = MAX_ASSEMBLED_DOMAIN_BYTES;
    let assembly_limit = MAX_ASSEMBLED_DOMAIN_BYTES_V46;
    let error = ASSEMBLED_DOMAIN_LIMIT_V46;

    assert_eq!(legacy_limit, 128 * MIB);
    assert_eq!(assembly_limit, 256 * MIB);
    assert_eq!(
        assembled_domain_limit(Profile::SourceExpandedV2, 512 * MIB),
        legacy_limit
    );
    assert_eq!(
        assembled_domain_limit(Profile::PlacementExpandedV1, 512 * MIB),
        legacy_limit
    );
    for profile in [Profile::AssemblyExpandedV1, Profile::GameObjectExpandedV1] {
        assert_eq!(assembled_domain_limit(profile, 512 * MIB), assembly_limit);
        assert_eq!(assembled_domain_limit(profile, 192 * MIB), 192 * MIB);
        assert_eq!(assembled_domain_limit(profile, 0), 0);
        assert_eq!(assembled_domain_error(profile), error);
    }
    assert_eq!(assembled_domain_error(Profile::PlacementExpandedV1), LIMIT);

    assert_eq!(
        assembled_domain_error(Profile::GameObjectExpandedV1),
        ASSEMBLED_DOMAIN_LIMIT_V46
    );
    assert_eq!(
        remaining_assembled_domain_bytes(assembly_limit, 64 * MIB, error).unwrap(),
        (assembly_limit - 64 * MIB) as usize
    );
    assert_eq!(
        remaining_assembled_domain_bytes(assembly_limit, assembly_limit, error).unwrap(),
        0
    );
    assert_eq!(
        remaining_assembled_domain_bytes(assembly_limit, assembly_limit + 1, error),
        Err(error)
    );
    assert_eq!(
        assembled_output_allowance(assembly_limit, 64 * MIB, (32 * MIB) as usize, error).unwrap(),
        (assembly_limit - 96 * MIB) as usize
    );
    assert_eq!(
        assembled_output_allowance(legacy_limit, 64 * MIB, (32 * MIB) as usize, LIMIT).unwrap(),
        (legacy_limit - 96 * MIB) as usize
    );
    assert_eq!(
        assembled_output_allowance(assembly_limit, assembly_limit, 0, error).unwrap(),
        0
    );
    assert_eq!(
        assembled_output_allowance(assembly_limit, assembly_limit, 1, error),
        Err(error)
    );
    assert_eq!(
        assembled_output_allowance(assembly_limit, assembly_limit + 1, 0, error),
        Err(error)
    );
    assert_eq!(
        assembled_output_allowance(assembly_limit, assembly_limit - 1, 2, error),
        Err(error)
    );
    assert_eq!(
        remaining_assembled_domain_bytes(u64::MAX, u64::MAX, error).unwrap(),
        0
    );
    if usize::BITS < 64 {
        assert_eq!(
            remaining_assembled_domain_bytes(u64::MAX, 0, error),
            Err(error)
        );
    }

    assert_eq!(
        next_parent_payload_bytes(legacy_limit as usize - 1, 1, legacy_limit, LIMIT).unwrap(),
        legacy_limit as usize
    );
    assert_eq!(
        next_parent_payload_bytes(legacy_limit as usize, 1, legacy_limit, LIMIT),
        Err(LIMIT)
    );
    assert_eq!(
        next_parent_payload_bytes(assembly_limit as usize - 1, 1, assembly_limit, error).unwrap(),
        assembly_limit as usize
    );
    assert_eq!(
        next_parent_payload_bytes(assembly_limit as usize, 1, assembly_limit, error),
        Err(error)
    );
    assert_eq!(
        next_parent_payload_bytes(usize::MAX, 1, assembly_limit, error),
        Err(error)
    );
    assert_eq!(
        next_parent_payload_capacity(legacy_limit as usize - 1, 1, legacy_limit as usize, LIMIT)
            .unwrap(),
        legacy_limit as usize
    );
    assert_eq!(
        next_parent_payload_capacity(
            assembly_limit as usize - 1,
            1,
            assembly_limit as usize,
            error
        )
        .unwrap(),
        assembly_limit as usize
    );
    assert_eq!(
        next_parent_payload_capacity(
            0,
            assembly_limit as usize + 1,
            assembly_limit as usize,
            error
        ),
        Err(error)
    );
    assert_eq!(
        next_parent_payload_capacity(assembly_limit as usize, 1, assembly_limit as usize, error),
        Err(error)
    );
    assert_eq!(
        next_parent_payload_capacity(usize::MAX, 1, usize::MAX, error),
        Err(error)
    );

    assert_eq!(
        add_assembled_domain_bytes(legacy_limit - 1, 1, LIMIT).unwrap(),
        legacy_limit
    );
    assert_eq!(
        add_assembled_domain_bytes(assembly_limit - 1, 1, error).unwrap(),
        assembly_limit
    );
    assert_eq!(add_assembled_domain_bytes(u64::MAX, 1, error), Err(error));
    assert_eq!(
        next_assembled_domain_total(legacy_limit - 1, 1, legacy_limit, LIMIT).unwrap(),
        legacy_limit
    );
    assert_eq!(
        next_assembled_domain_total(legacy_limit, 1, legacy_limit, LIMIT),
        Err(LIMIT)
    );
    assert_eq!(
        next_assembled_domain_total(assembly_limit - 1, 1, assembly_limit, error).unwrap(),
        assembly_limit
    );
    assert_eq!(
        next_assembled_domain_total(assembly_limit, 1, assembly_limit, error),
        Err(error)
    );
    assert_eq!(
        next_assembled_domain_total(u64::MAX, 1, u64::MAX, error),
        Err(error)
    );
}

#[test]
fn coverage_v18_rejects_crossed_resources_and_noncanonical_or_overlapping_lists() {
    let coverage = placement_expanded_material_filtered_wmo_sample();
    let bytes = coverage.encode().unwrap();
    for resource in [
        resources::Profile::RequestHeavyV1,
        resources::Profile::DependencyHeavyV1,
        resources::Profile::SourceHeavyV1,
        resources::Profile::ReadHeavyV1,
        resources::Profile::ModelHeavyV1,
        resources::Profile::SourceExpandedV1,
        resources::Profile::ModelOutputExpandedV1,
        resources::Profile::ModelBytesExpandedV1,
        resources::Profile::SourceExpandedV2,
        resources::Profile::BoundedV1,
        resources::Profile::DiskWorldV1,
    ] {
        let crossed = replace_text(
            &bytes,
            resources::Profile::PlacementExpandedV1.id(),
            resource.id(),
        );
        assert!(Coverage::parse_v45(&crossed).is_err());
    }
    for (identity, replacement) in [
        (
            PLACEMENT_EXPANDED_MATERIAL_FILTERED_WMO_CODEC_PROFILE,
            SOURCE_EXPANDED_V2_MATERIAL_FILTERED_WMO_CODEC_PROFILE,
        ),
        (
            PLACEMENT_EXPANDED_MATERIAL_FILTERED_WMO_DOMAIN_PROFILE,
            SOURCE_EXPANDED_V2_MATERIAL_FILTERED_WMO_DOMAIN_PROFILE,
        ),
    ] {
        assert!(Coverage::parse_v45(&replace_text(&bytes, identity, replacement)).is_err());
    }
    for invalid in [vec![0], vec![9, 8], vec![9, 9]] {
        let mut invalid_ids = placement_expanded_material_filtered_wmo_sample();
        invalid_ids.material_filtered_wmo = invalid;
        assert!(invalid_ids.encode().is_err());
    }
    let mut overlap = placement_expanded_material_filtered_wmo_sample();
    overlap.material_filtered_wmo[0] = overlap.missing_m2[1];
    assert!(overlap.encode().is_err());
}

#[test]
fn coverage_v15_rejects_crossed_resources_and_noncanonical_or_overlapping_lists() {
    let coverage = model_output_expanded_material_filtered_wmo_sample();
    let bytes = coverage.encode().unwrap();
    for resource in [
        resources::Profile::RequestHeavyV1,
        resources::Profile::DependencyHeavyV1,
        resources::Profile::SourceHeavyV1,
        resources::Profile::ReadHeavyV1,
        resources::Profile::ModelHeavyV1,
        resources::Profile::SourceExpandedV1,
        resources::Profile::BoundedV1,
        resources::Profile::DiskWorldV1,
    ] {
        let crossed = replace_text(
            &bytes,
            resources::Profile::ModelOutputExpandedV1.id(),
            resource.id(),
        );
        assert!(Coverage::parse_v42(&crossed).is_err());
    }
    let wrong_profile = replace_text(
        &bytes,
        MODEL_OUTPUT_EXPANDED_MATERIAL_FILTERED_WMO_CODEC_PROFILE,
        SOURCE_EXPANDED_MATERIAL_FILTERED_WMO_CODEC_PROFILE,
    );
    assert!(Coverage::parse_v42(&wrong_profile).is_err());

    let mut limited = model_output_expanded_material_filtered_wmo_sample();
    limited.material_filtered_wmo = (1..=COVERAGE_MAX_MATERIAL_FILTERED_WMO as u32 + 1).collect();
    assert_eq!(limited.encode().unwrap_err().code, LIMIT);
    for invalid in [vec![0], vec![9, 8], vec![9, 9]] {
        let mut invalid_ids = model_output_expanded_material_filtered_wmo_sample();
        invalid_ids.material_filtered_wmo = invalid;
        assert!(invalid_ids.encode().is_err());
    }
    let mut overlap = model_output_expanded_material_filtered_wmo_sample();
    overlap.material_filtered_wmo[0] = overlap.missing_m2[1];
    assert!(overlap.encode().is_err());
}

#[test]
fn coverage_v14_rejects_crossed_resources_and_noncanonical_or_overlapping_lists() {
    let coverage = source_expanded_material_filtered_wmo_sample();
    let bytes = coverage.encode().unwrap();
    for resource in [
        resources::Profile::RequestHeavyV1,
        resources::Profile::DependencyHeavyV1,
        resources::Profile::SourceHeavyV1,
        resources::Profile::ReadHeavyV1,
        resources::Profile::ModelHeavyV1,
        resources::Profile::BoundedV1,
        resources::Profile::DiskWorldV1,
    ] {
        let crossed = replace_text(
            &bytes,
            resources::Profile::SourceExpandedV1.id(),
            resource.id(),
        );
        assert!(Coverage::parse_v41(&crossed).is_err());
    }
    let wrong_profile = replace_text(
        &bytes,
        SOURCE_EXPANDED_MATERIAL_FILTERED_WMO_CODEC_PROFILE,
        MODEL_HEAVY_MATERIAL_FILTERED_WMO_CODEC_PROFILE,
    );
    assert!(Coverage::parse_v41(&wrong_profile).is_err());

    let mut limited = source_expanded_material_filtered_wmo_sample();
    limited.material_filtered_wmo = (1..=COVERAGE_MAX_MATERIAL_FILTERED_WMO as u32 + 1).collect();
    assert_eq!(limited.encode().unwrap_err().code, LIMIT);
    for invalid in [vec![0], vec![9, 8], vec![9, 9]] {
        let mut invalid_ids = source_expanded_material_filtered_wmo_sample();
        invalid_ids.material_filtered_wmo = invalid;
        assert!(invalid_ids.encode().is_err());
    }
    let mut overlap = source_expanded_material_filtered_wmo_sample();
    overlap.material_filtered_wmo[0] = overlap.missing_m2[1];
    assert!(overlap.encode().is_err());
}

#[test]
fn coverage_v13_rejects_crossed_resources_and_noncanonical_or_overlapping_lists() {
    let coverage = model_heavy_material_filtered_wmo_sample();
    let bytes = coverage.encode().unwrap();
    for resource in [
        resources::Profile::RequestHeavyV1,
        resources::Profile::DependencyHeavyV1,
        resources::Profile::SourceHeavyV1,
        resources::Profile::ReadHeavyV1,
        resources::Profile::BoundedV1,
        resources::Profile::DiskWorldV1,
    ] {
        let crossed = replace_text(&bytes, resources::Profile::ModelHeavyV1.id(), resource.id());
        assert!(Coverage::parse_v40(&crossed).is_err());
    }

    let mut limited = model_heavy_material_filtered_wmo_sample();
    limited.material_filtered_wmo = (1..=COVERAGE_MAX_MATERIAL_FILTERED_WMO as u32 + 1).collect();
    assert_eq!(limited.encode().unwrap_err().code, LIMIT);
    for invalid in [vec![0], vec![9, 8], vec![9, 9]] {
        let mut invalid_ids = model_heavy_material_filtered_wmo_sample();
        invalid_ids.material_filtered_wmo = invalid;
        assert!(invalid_ids.encode().is_err());
    }
    let mut overlap = model_heavy_material_filtered_wmo_sample();
    overlap.material_filtered_wmo[0] = overlap.missing_m2[1];
    assert!(overlap.encode().is_err());
    assert_eq!(COVERAGE_MAX_MISSING_M2, 4096);
    assert_eq!(COVERAGE_MAX_MISSING_WMO, 4096);
    assert_eq!(COVERAGE_MAX_EMPTY_WMO, 4096);
    assert_eq!(COVERAGE_MAX_UNSELECTED_WMO, 4096);
    assert_eq!(COVERAGE_MAX_MATERIAL_FILTERED_WMO, 4096);
}

#[test]
fn coverage_parsers_require_exact_versions_selections_and_component_masks() {
    let v22 = sample();
    assert_eq!(v22.context.components, 16);
    let v22_bytes = v22.encode().unwrap();
    assert_eq!(u32::from_le_bytes(v22_bytes[8..12].try_into().unwrap()), 1);
    assert!(Coverage::parse(&v22_bytes).is_ok());
    assert!(Coverage::parse_v28(&v22_bytes).is_err());
    assert!(Coverage::parse_v29(&v22_bytes).is_err());
    assert!(Coverage::parse_v30(&v22_bytes).is_err());
    assert!(Coverage::parse_v31(&v22_bytes).is_err());
    assert!(Coverage::parse_v32(&v22_bytes).is_err());

    let mut v28 = sample();
    v28.context.components = Component::Db2.bit() | Component::Collision.bit();
    assert_eq!(v28.context.components, 17);
    let v28_bytes = v28.encode().unwrap();
    assert!(Coverage::parse(&v28_bytes).is_err());
    assert!(Coverage::parse_v29(&v28_bytes).is_err());
    assert!(Coverage::parse_v30(&v28_bytes).is_err());
    assert!(Coverage::parse_v31(&v28_bytes).is_err());
    assert!(Coverage::parse_v32(&v28_bytes).is_err());
    let parsed = Coverage::parse_v28(&v28_bytes).unwrap();
    assert_eq!(parsed.context, v28.context);
    assert_eq!(parsed.selection, MapSelection::MapAndAncestors(2));

    let v29 = available_sample();
    assert_eq!(v29.context.components, 17);
    let v29_bytes = v29.encode().unwrap();
    assert_eq!(u32::from_le_bytes(v29_bytes[8..12].try_into().unwrap()), 2);
    assert!(Coverage::parse(&v29_bytes).is_err());
    assert!(Coverage::parse_v28(&v29_bytes).is_err());
    assert!(Coverage::parse_v30(&v29_bytes).is_err());
    assert!(Coverage::parse_v31(&v29_bytes).is_err());
    assert!(Coverage::parse_v32(&v29_bytes).is_err());
    let parsed = Coverage::parse_v29(&v29_bytes).unwrap();
    assert_eq!(parsed.context, v29.context);
    assert_eq!(parsed.selection, MapSelection::All);

    let v30 = root_absent_sample();
    let v30_bytes = v30.encode().unwrap();
    assert!(Coverage::parse(&v30_bytes).is_err());
    assert!(Coverage::parse_v28(&v30_bytes).is_err());
    assert!(Coverage::parse_v29(&v30_bytes).is_err());
    assert!(Coverage::parse_v31(&v30_bytes).is_err());
    assert!(Coverage::parse_v32(&v30_bytes).is_err());
    assert_eq!(
        Coverage::parse_v30(&v30_bytes).unwrap().missing_wdt_policy,
        run::MissingWdtPolicy::DeclareRootAbsent
    );

    let v31 = model_root_absent_sample();
    let v31_bytes = v31.encode().unwrap();
    assert!(Coverage::parse(&v31_bytes).is_err());
    assert!(Coverage::parse_v28(&v31_bytes).is_err());
    assert!(Coverage::parse_v29(&v31_bytes).is_err());
    assert!(Coverage::parse_v30(&v31_bytes).is_err());
    assert!(Coverage::parse_v32(&v31_bytes).is_err());
    assert_eq!(
        Coverage::parse_v31(&v31_bytes).unwrap().missing_m2_policy,
        run::MissingModelPolicy::DeclareRootAbsentWmoDoodadM2
    );

    let v32 = request_capacity_sample();
    let v32_bytes = v32.encode().unwrap();
    assert!(Coverage::parse(&v32_bytes).is_err());
    assert!(Coverage::parse_v28(&v32_bytes).is_err());
    assert!(Coverage::parse_v29(&v32_bytes).is_err());
    assert!(Coverage::parse_v30(&v32_bytes).is_err());
    assert!(Coverage::parse_v31(&v32_bytes).is_err());
    assert_eq!(
        Coverage::parse_v32(&v32_bytes).unwrap().resource_profile,
        resources::Profile::RequestHeavyV1
    );

    let v33 = vacated_wmo_sample();
    let v33_bytes = v33.encode().unwrap();
    assert!(Coverage::parse(&v33_bytes).is_err());
    assert!(Coverage::parse_v28(&v33_bytes).is_err());
    assert!(Coverage::parse_v29(&v33_bytes).is_err());
    assert!(Coverage::parse_v30(&v33_bytes).is_err());
    assert!(Coverage::parse_v31(&v33_bytes).is_err());
    assert!(Coverage::parse_v32(&v33_bytes).is_err());
    assert_eq!(
        Coverage::parse_v33(&v33_bytes).unwrap().missing_wmo_policy,
        run::MissingWmoPolicy::DeclareVacatedAfterRootAbsentM2
    );

    let v34 = empty_wmo_sample();
    let v34_bytes = v34.encode().unwrap();
    assert!(Coverage::parse(&v34_bytes).is_err());
    assert!(Coverage::parse_v28(&v34_bytes).is_err());
    assert!(Coverage::parse_v29(&v34_bytes).is_err());
    assert!(Coverage::parse_v30(&v34_bytes).is_err());
    assert!(Coverage::parse_v31(&v34_bytes).is_err());
    assert!(Coverage::parse_v32(&v34_bytes).is_err());
    assert!(Coverage::parse_v33(&v34_bytes).is_err());
    assert_eq!(
        Coverage::parse_v34(&v34_bytes).unwrap().missing_wmo_policy,
        run::MissingWmoPolicy::DeclareVacatedAfterRootAbsentM2OrEmpty
    );

    let v35 = unselected_wmo_sample();
    let v35_bytes = v35.encode().unwrap();
    assert!(Coverage::parse(&v35_bytes).is_err());
    assert!(Coverage::parse_v28(&v35_bytes).is_err());
    assert!(Coverage::parse_v29(&v35_bytes).is_err());
    assert!(Coverage::parse_v30(&v35_bytes).is_err());
    assert!(Coverage::parse_v31(&v35_bytes).is_err());
    assert!(Coverage::parse_v32(&v35_bytes).is_err());
    assert!(Coverage::parse_v33(&v35_bytes).is_err());
    assert!(Coverage::parse_v34(&v35_bytes).is_err());
    assert_eq!(
        Coverage::parse_v35(&v35_bytes).unwrap().missing_wmo_policy,
        run::MissingWmoPolicy::DeclareVacatedAfterRootAbsentM2OrEmptyOrUnselected
    );

    let v36 = material_filtered_wmo_sample();
    let v36_bytes = v36.encode().unwrap();
    assert!(Coverage::parse(&v36_bytes).is_err());
    assert!(Coverage::parse_v28(&v36_bytes).is_err());
    assert!(Coverage::parse_v29(&v36_bytes).is_err());
    assert!(Coverage::parse_v30(&v36_bytes).is_err());
    assert!(Coverage::parse_v31(&v36_bytes).is_err());
    assert!(Coverage::parse_v32(&v36_bytes).is_err());
    assert!(Coverage::parse_v33(&v36_bytes).is_err());
    assert!(Coverage::parse_v34(&v36_bytes).is_err());
    assert!(Coverage::parse_v35(&v36_bytes).is_err());
    assert_eq!(
        Coverage::parse_v36(&v36_bytes).unwrap().missing_wmo_policy,
        run::MissingWmoPolicy::DeclareVacatedAfterRootAbsentM2OrEmptyOrUnselectedOrMaterialFiltered
    );

    let mut wrong_mask = request_capacity_sample();
    wrong_mask.context.components = Component::Collision.bit();
    let wrong_mask = wrong_mask.encode().unwrap();
    assert!(Coverage::parse(&wrong_mask).is_err());
    assert!(Coverage::parse_v28(&wrong_mask).is_err());
    assert!(Coverage::parse_v29(&wrong_mask).is_err());
    assert!(Coverage::parse_v30(&wrong_mask).is_err());
    assert!(Coverage::parse_v31(&wrong_mask).is_err());
    assert!(Coverage::parse_v32(&wrong_mask).is_err());
}

#[test]
fn available_v2_has_no_map_id_and_zero_is_never_a_selection_sentinel() {
    let mut available = available_sample();
    available.maps[0].id = 0;
    let bytes = available.encode().unwrap();
    let at = after_context(&bytes);
    let profile = crate::plan::AVAILABLE_GEOMETRY_COVERAGE_PROFILE;
    let length = u32::from_le_bytes(bytes[at..at + 4].try_into().unwrap()) as usize;
    assert_eq!(length, profile.len());
    assert_eq!(&bytes[at + 4..at + 4 + length], profile.as_bytes());
    let parsed = Coverage::parse_v29(&bytes).unwrap();
    assert_eq!(parsed.selection, MapSelection::All);
    assert_eq!(parsed.maps[0].id, 0);

    let mut selected_zero = sample();
    selected_zero.selection = MapSelection::MapAndAncestors(0);
    selected_zero.maps[0].id = 0;
    let selected_bytes = selected_zero.encode().unwrap();
    let selected_at = after_context(&selected_bytes);
    assert_eq!(
        u32::from_le_bytes(selected_bytes[8..12].try_into().unwrap()),
        1
    );
    assert_eq!(
        u32::from_le_bytes(
            selected_bytes[selected_at..selected_at + 4]
                .try_into()
                .unwrap()
        ),
        0
    );
    assert_eq!(
        Coverage::parse(&selected_bytes).unwrap().selection,
        MapSelection::MapAndAncestors(0)
    );

    let mut empty_selected = sample();
    empty_selected.maps.clear();
    assert!(Coverage::parse(&empty_selected.encode().unwrap()).is_err());
    let mut empty_available = available_sample();
    empty_available.maps.clear();
    let parsed = Coverage::parse_v29(&empty_available.encode().unwrap()).unwrap();
    assert_eq!(parsed.selection, MapSelection::All);
    assert!(parsed.maps.is_empty());

    let mut empty_root_absent = root_absent_sample();
    empty_root_absent.maps.clear();
    let bytes = empty_root_absent.encode().unwrap();
    assert_eq!(
        u32::from_le_bytes(bytes[8..12].try_into().unwrap()),
        ROOT_ABSENT_COVERAGE_VERSION
    );
    let parsed = Coverage::parse_v30(&bytes).unwrap();
    assert_eq!(parsed.selection, MapSelection::All);
    assert_eq!(
        parsed.missing_wdt_policy,
        run::MissingWdtPolicy::DeclareRootAbsent
    );
    assert!(parsed.maps.is_empty());

    let mut empty_model_root_absent = model_root_absent_sample();
    empty_model_root_absent.maps.clear();
    empty_model_root_absent.missing_m2.clear();
    let bytes = empty_model_root_absent.encode().unwrap();
    assert_eq!(
        u32::from_le_bytes(bytes[8..12].try_into().unwrap()),
        MODEL_ROOT_ABSENT_COVERAGE_VERSION
    );
    let parsed = Coverage::parse_v31(&bytes).unwrap();
    assert_eq!(parsed.selection, MapSelection::All);
    assert_eq!(
        parsed.missing_m2_policy,
        run::MissingModelPolicy::DeclareRootAbsentWmoDoodadM2
    );
    assert!(parsed.maps.is_empty());
    assert!(parsed.missing_m2.is_empty());

    let mut empty_request_capacity = request_capacity_sample();
    empty_request_capacity.maps.clear();
    empty_request_capacity.missing_m2.clear();
    let bytes = empty_request_capacity.encode().unwrap();
    assert_eq!(
        u32::from_le_bytes(bytes[8..12].try_into().unwrap()),
        REQUEST_CAPACITY_COVERAGE_VERSION
    );
    let parsed = Coverage::parse_v32(&bytes).unwrap();
    assert_eq!(parsed.selection, MapSelection::All);
    assert_eq!(parsed.resource_profile, resources::Profile::RequestHeavyV1);
    assert!(parsed.maps.is_empty());
    assert!(parsed.missing_m2.is_empty());

    let mut empty_vacated = vacated_wmo_sample();
    empty_vacated.maps.clear();
    empty_vacated.missing_m2.clear();
    empty_vacated.missing_wmo.clear();
    let bytes = empty_vacated.encode().unwrap();
    let parsed = Coverage::parse_v33(&bytes).unwrap();
    assert_eq!(parsed.selection, MapSelection::All);
    assert!(parsed.maps.is_empty());
    assert!(parsed.missing_m2.is_empty());
    assert!(parsed.missing_wmo.is_empty());

    let mut empty_empty_wmo = empty_wmo_sample();
    empty_empty_wmo.maps.clear();
    empty_empty_wmo.missing_m2.clear();
    empty_empty_wmo.missing_wmo.clear();
    empty_empty_wmo.empty_wmo.clear();
    let parsed = Coverage::parse_v34(&empty_empty_wmo.encode().unwrap()).unwrap();
    assert!(parsed.maps.is_empty());
    assert!(parsed.missing_m2.is_empty());
    assert!(parsed.missing_wmo.is_empty());
    assert!(parsed.empty_wmo.is_empty());
    assert!(parsed.unselected_wmo.is_empty());

    let mut empty_unselected_wmo = unselected_wmo_sample();
    empty_unselected_wmo.maps.clear();
    empty_unselected_wmo.missing_m2.clear();
    empty_unselected_wmo.missing_wmo.clear();
    empty_unselected_wmo.empty_wmo.clear();
    empty_unselected_wmo.unselected_wmo.clear();
    let parsed = Coverage::parse_v35(&empty_unselected_wmo.encode().unwrap()).unwrap();
    assert!(parsed.maps.is_empty());
    assert!(parsed.missing_m2.is_empty());
    assert!(parsed.missing_wmo.is_empty());
    assert!(parsed.empty_wmo.is_empty());
    assert!(parsed.unselected_wmo.is_empty());

    let mut empty_material_filtered_wmo = material_filtered_wmo_sample();
    empty_material_filtered_wmo.maps.clear();
    empty_material_filtered_wmo.missing_m2.clear();
    empty_material_filtered_wmo.missing_wmo.clear();
    empty_material_filtered_wmo.empty_wmo.clear();
    empty_material_filtered_wmo.unselected_wmo.clear();
    empty_material_filtered_wmo.material_filtered_wmo.clear();
    let parsed = Coverage::parse_v36(&empty_material_filtered_wmo.encode().unwrap()).unwrap();
    assert!(parsed.maps.is_empty());
    assert!(parsed.missing_m2.is_empty());
    assert!(parsed.missing_wmo.is_empty());
    assert!(parsed.empty_wmo.is_empty());
    assert!(parsed.unselected_wmo.is_empty());
    assert!(parsed.material_filtered_wmo.is_empty());
}

#[test]
fn coverage_rejects_crossed_versions_unknown_versions_and_altered_profiles() {
    let selected = sample().encode().unwrap();
    let mut selected_as_v2 = selected.clone();
    selected_as_v2[8..12].copy_from_slice(&2_u32.to_le_bytes());
    assert!(Coverage::parse_v29(&selected_as_v2).is_err());
    let mut selected_as_v3 = selected.clone();
    selected_as_v3[8..12].copy_from_slice(&3_u32.to_le_bytes());
    assert!(Coverage::parse_v30(&selected_as_v3).is_err());

    let available = available_sample().encode().unwrap();
    let mut available_as_v1 = available.clone();
    available_as_v1[8..12].copy_from_slice(&1_u32.to_le_bytes());
    assert!(Coverage::parse(&available_as_v1).is_err());
    assert!(Coverage::parse_v28(&available_as_v1).is_err());
    let mut available_as_v3 = available.clone();
    available_as_v3[8..12].copy_from_slice(&3_u32.to_le_bytes());
    assert!(Coverage::parse_v30(&available_as_v3).is_err());

    let root_absent = root_absent_sample().encode().unwrap();
    let model_root_absent = model_root_absent_sample().encode().unwrap();
    let request_capacity = request_capacity_sample().encode().unwrap();
    let vacated_wmo = vacated_wmo_sample().encode().unwrap();
    let unselected_wmo = unselected_wmo_sample().encode().unwrap();
    for version in [0, 6, u32::MAX] {
        let mut unknown = selected.clone();
        unknown[8..12].copy_from_slice(&version.to_le_bytes());
        assert!(Coverage::parse(&unknown).is_err());
        assert!(Coverage::parse_v28(&unknown).is_err());
        assert!(Coverage::parse_v29(&unknown).is_err());
        assert!(Coverage::parse_v30(&unknown).is_err());
        assert!(Coverage::parse_v31(&unknown).is_err());
        assert!(Coverage::parse_v32(&unknown).is_err());
        assert!(Coverage::parse_v33(&unknown).is_err());
        assert!(Coverage::parse_v34(&unknown).is_err());
        assert!(Coverage::parse_v35(&unknown).is_err());
    }
    for text in [
        crate::plan::SELECTED_GEOMETRY_COVERAGE_PROFILE,
        PROFILE,
        "available-map-active-ancestors-v1",
    ] {
        assert!(Coverage::parse(&mutate_text(&selected, text)).is_err());
    }
    for text in [
        crate::plan::AVAILABLE_GEOMETRY_COVERAGE_PROFILE,
        AVAILABLE_COVERAGE_PROFILE,
        "available-map-all-active-v1",
    ] {
        assert!(Coverage::parse_v29(&mutate_text(&available, text)).is_err());
    }
    for text in [
        crate::plan::ROOT_ABSENT_GEOMETRY_COVERAGE_PROFILE,
        ROOT_ABSENT_COVERAGE_PROFILE,
        ROOT_ABSENT_DOMAIN_PROFILE,
    ] {
        assert!(Coverage::parse_v30(&mutate_text(&root_absent, text)).is_err());
    }
    for text in [
        crate::plan::MODEL_ROOT_ABSENT_GEOMETRY_COVERAGE_PROFILE,
        MODEL_ROOT_ABSENT_COVERAGE_PROFILE,
        MODEL_ROOT_ABSENT_DOMAIN_PROFILE,
    ] {
        assert!(Coverage::parse_v31(&mutate_text(&model_root_absent, text)).is_err());
    }
    for text in [
        crate::plan::MODEL_ROOT_ABSENT_REQUEST_CAPACITY_GEOMETRY_COVERAGE_PROFILE,
        REQUEST_CAPACITY_COVERAGE_PROFILE,
        REQUEST_CAPACITY_DOMAIN_PROFILE,
    ] {
        assert!(Coverage::parse_v32(&mutate_text(&request_capacity, text)).is_err());
    }
    for text in [
        crate::plan::VACATED_WMO_COVERAGE_PROFILE,
        VACATED_WMO_CODEC_PROFILE,
        VACATED_WMO_DOMAIN_PROFILE,
    ] {
        assert!(Coverage::parse_v33(&mutate_text(&vacated_wmo, text)).is_err());
    }
    let empty_wmo = empty_wmo_sample().encode().unwrap();
    for text in [
        crate::plan::EMPTY_WMO_COVERAGE_PROFILE,
        EMPTY_WMO_CODEC_PROFILE,
        EMPTY_WMO_DOMAIN_PROFILE,
    ] {
        assert!(Coverage::parse_v34(&mutate_text(&empty_wmo, text)).is_err());
    }
    for text in [
        crate::plan::UNSELECTED_WMO_COVERAGE_PROFILE,
        UNSELECTED_WMO_CODEC_PROFILE,
        UNSELECTED_WMO_DOMAIN_PROFILE,
    ] {
        assert!(Coverage::parse_v35(&mutate_text(&unselected_wmo, text)).is_err());
    }

    let at = after_context(&root_absent);
    let profile = crate::plan::ROOT_ABSENT_GEOMETRY_COVERAGE_PROFILE;
    let length = u32::from_le_bytes(root_absent[at..at + 4].try_into().unwrap()) as usize;
    assert_eq!(&root_absent[at + 4..at + 4 + length], profile.as_bytes());
    let mut policy_at = at;
    for _ in 0..3 {
        let length =
            u32::from_le_bytes(root_absent[policy_at..policy_at + 4].try_into().unwrap()) as usize;
        policy_at += 4 + length;
    }
    assert_eq!(root_absent[policy_at], 1);
    for invalid in [0, 2] {
        let mut mutated = root_absent.clone();
        mutated[policy_at] = invalid;
        assert!(Coverage::parse_v30(&mutated).is_err());
    }

    let mut missing_selected = sample();
    missing_selected.selection = MapSelection::MapAndAncestors(3);
    assert!(Coverage::parse(&missing_selected.encode().unwrap()).is_err());
    let mut selected_root_policy = root_absent_sample();
    selected_root_policy.selection = MapSelection::MapAndAncestors(2);
    assert!(selected_root_policy.encode().is_err());
    let mut old_root_state = available_sample();
    old_root_state.maps[0].root_absent = true;
    assert!(old_root_state.encode().is_err());
    let mut old_missing_m2 = model_root_absent_sample();
    old_missing_m2.missing_m2_policy = run::MissingModelPolicy::Reject;
    assert!(old_missing_m2.encode().is_err());
}

#[test]
fn coverage_v6_rejects_crossed_resources_policies_and_malformed_wmo_lists() {
    let mut bad_policy = vacated_wmo_sample();
    bad_policy.missing_wmo_policy = run::MissingWmoPolicy::Reject;
    assert!(bad_policy.encode().is_err());

    let mut bad_missing_m2_policy = vacated_wmo_sample();
    bad_missing_m2_policy.missing_m2_policy = run::MissingModelPolicy::Reject;
    assert!(bad_missing_m2_policy.encode().is_err());

    let mut bad_missing_wdt_policy = vacated_wmo_sample();
    bad_missing_wdt_policy.missing_wdt_policy = run::MissingWdtPolicy::Reject;
    assert!(bad_missing_wdt_policy.encode().is_err());

    for profile in [
        resources::Profile::BoundedV1,
        resources::Profile::DiskWorldV1,
    ] {
        let mut crossed = vacated_wmo_sample();
        crossed.resource_profile = profile;
        assert!(crossed.encode().is_err());
    }
    let mut selected = vacated_wmo_sample();
    selected.selection = MapSelection::MapAndAncestors(2);
    assert!(selected.encode().is_err());

    let valid = vacated_wmo_sample();
    let valid_bytes = valid.encode().unwrap();
    let first = valid.missing_wmo[0].to_le_bytes();
    let resource = resources::Profile::RequestHeavyV1.id();
    let resource_at = valid_bytes
        .windows(resource.len())
        .position(|window| window == resource.as_bytes())
        .unwrap();
    let first_at = resource_at + resource.len() + 1 + 4;
    assert_eq!(valid_bytes[first_at..first_at + 4], first);
    let second_at = first_at + 4;
    for invalid_id in [0, valid.missing_wmo[1]] {
        let mut mutated = valid_bytes.clone();
        mutated[first_at..first_at + 4].copy_from_slice(&invalid_id.to_le_bytes());
        assert!(Coverage::parse_v33(&mutated).is_err());
    }
    let mut duplicate = valid_bytes.clone();
    duplicate[second_at..second_at + 4].copy_from_slice(&first);
    assert!(Coverage::parse_v33(&duplicate).is_err());
    let mut unordered = valid_bytes.clone();
    unordered[second_at..second_at + 4].copy_from_slice(&1_u32.to_le_bytes());
    assert!(Coverage::parse_v33(&unordered).is_err());
    assert!(
        validate_missing_wmo(
            &vec![1; COVERAGE_MAX_MISSING_WMO + 1],
            run::MissingWmoPolicy::DeclareVacatedAfterRootAbsentM2,
            &valid.missing_m2
        )
        .is_err()
    );

    let mut overlapping = valid;
    overlapping.missing_wmo[0] = overlapping.missing_m2[0];
    assert!(overlapping.encode().is_err());
}

#[test]
fn coverage_v7_enforces_separate_sorted_bounded_nonzero_nonoverlapping_lists() {
    let coverage = empty_wmo_sample();
    assert_eq!(
        validate_empty_wmo(
            &coverage.empty_wmo,
            coverage.missing_wmo_policy,
            &coverage.missing_wmo,
            &coverage.missing_m2,
        ),
        Ok(())
    );
    for invalid in [vec![0], vec![9, 8], vec![9, 9]] {
        assert!(
            validate_empty_wmo(
                &invalid,
                coverage.missing_wmo_policy,
                &coverage.missing_wmo,
                &coverage.missing_m2,
            )
            .is_err()
        );
    }
    assert_eq!(
        validate_empty_wmo(
            &vec![1; COVERAGE_MAX_EMPTY_WMO + 1],
            coverage.missing_wmo_policy,
            &coverage.missing_wmo,
            &coverage.missing_m2,
        )
        .unwrap_err()
        .code,
        LIMIT
    );
    assert!(
        validate_empty_wmo(
            &[coverage.missing_wmo[0]],
            coverage.missing_wmo_policy,
            &coverage.missing_wmo,
            &coverage.missing_m2,
        )
        .is_err()
    );
    assert!(
        validate_empty_wmo(
            &[coverage.missing_m2[0]],
            coverage.missing_wmo_policy,
            &coverage.missing_wmo,
            &coverage.missing_m2,
        )
        .is_err()
    );
    assert!(validate_empty_wmo(&[1], run::MissingWmoPolicy::Reject, &[], &[],).is_err());

    let valid_bytes = coverage.encode().unwrap();
    let resource = resources::Profile::RequestHeavyV1.id();
    let resource_at = valid_bytes
        .windows(resource.len())
        .position(|window| window == resource.as_bytes())
        .unwrap();
    let missing_count_at = resource_at + resource.len() + 1;
    let missing_count = u32::from_le_bytes(
        valid_bytes[missing_count_at..missing_count_at + 4]
            .try_into()
            .unwrap(),
    ) as usize;
    let empty_count_at = missing_count_at + 4 + missing_count * 4;
    let empty_count = u32::from_le_bytes(
        valid_bytes[empty_count_at..empty_count_at + 4]
            .try_into()
            .unwrap(),
    ) as usize;
    assert_eq!(empty_count, coverage.empty_wmo.len());
    let mut too_many_missing = valid_bytes.clone();
    too_many_missing[missing_count_at..missing_count_at + 4]
        .copy_from_slice(&((COVERAGE_MAX_MISSING_WMO + 1) as u32).to_le_bytes());
    assert!(Coverage::parse_v34(&too_many_missing).is_err());
    let mut too_many_empty = valid_bytes.clone();
    too_many_empty[empty_count_at..empty_count_at + 4]
        .copy_from_slice(&((COVERAGE_MAX_EMPTY_WMO + 1) as u32).to_le_bytes());
    assert!(Coverage::parse_v34(&too_many_empty).is_err());
    let first_id_at = empty_count_at + 4;
    let first_id = coverage.empty_wmo[0].to_le_bytes();
    assert_eq!(&valid_bytes[first_id_at..first_id_at + 4], &first_id);
    let second_id_at = first_id_at + 4;
    for invalid_id in [0, coverage.empty_wmo[1]] {
        let mut mutated = valid_bytes.clone();
        mutated[first_id_at..first_id_at + 4].copy_from_slice(&invalid_id.to_le_bytes());
        assert!(Coverage::parse_v34(&mutated).is_err());
    }
    let mut out_of_order = valid_bytes.clone();
    out_of_order[second_id_at..second_id_at + 4].copy_from_slice(&1_u32.to_le_bytes());
    assert!(Coverage::parse_v34(&out_of_order).is_err());
    let mut overlaps_missing = valid_bytes.clone();
    overlaps_missing[first_id_at..first_id_at + 4]
        .copy_from_slice(&coverage.missing_wmo[0].to_le_bytes());
    assert!(Coverage::parse_v34(&overlaps_missing).is_err());
    let mut overlaps_m2 = valid_bytes.clone();
    overlaps_m2[first_id_at..first_id_at + 4]
        .copy_from_slice(&coverage.missing_m2[0].to_le_bytes());
    assert!(Coverage::parse_v34(&overlaps_m2).is_err());
    let mut false_policy = valid_bytes;
    false_policy[resource_at + resource.len()] = 0;
    assert!(Coverage::parse_v34(&false_policy).is_err());
}

#[test]
fn coverage_v9_accepts_all_five_omission_lists_at_their_exact_limit() {
    let ids =
        |first: u32| (first..first + COVERAGE_MAX_MATERIAL_FILTERED_WMO as u32).collect::<Vec<_>>();
    let full_coverage = || {
        let mut coverage = material_filtered_wmo_sample();
        coverage.missing_m2 = ids(1);
        coverage.missing_wmo = ids(4097);
        coverage.empty_wmo = ids(8193);
        coverage.unselected_wmo = ids(12289);
        coverage.material_filtered_wmo = ids(16385);
        coverage
    };
    let coverage = full_coverage();

    let encoded = coverage.encode().unwrap();
    let parsed = Coverage::parse_v36(&encoded).unwrap();
    assert_eq!(parsed.missing_m2.len(), COVERAGE_MAX_MISSING_M2);
    assert_eq!(parsed.missing_wmo.len(), COVERAGE_MAX_MISSING_WMO);
    assert_eq!(parsed.empty_wmo.len(), COVERAGE_MAX_EMPTY_WMO);
    assert_eq!(parsed.unselected_wmo.len(), COVERAGE_MAX_UNSELECTED_WMO);
    assert_eq!(
        parsed.material_filtered_wmo.len(),
        COVERAGE_MAX_MATERIAL_FILTERED_WMO
    );

    let mut overfull = full_coverage();
    overfull.missing_m2.push(20481);
    assert!(overfull.encode().is_err());
    let mut overfull = full_coverage();
    overfull.missing_wmo.push(20481);
    assert!(overfull.encode().is_err());
    let mut overfull = full_coverage();
    overfull.empty_wmo.push(20481);
    assert!(overfull.encode().is_err());
    let mut overfull = full_coverage();
    overfull.unselected_wmo.push(20481);
    assert!(overfull.encode().is_err());
    let mut overfull = full_coverage();
    overfull.material_filtered_wmo.push(20481);
    assert!(overfull.encode().is_err());
}

#[test]
fn coverage_v9_enforces_bounded_sorted_nonzero_disjoint_material_filtered_wmo_ids() {
    let coverage = material_filtered_wmo_sample();
    assert_eq!(
        validate_material_filtered_wmo(
            &coverage.material_filtered_wmo,
            coverage.missing_wmo_policy,
            &coverage.missing_m2,
            &coverage.missing_wmo,
            &coverage.empty_wmo,
            &coverage.unselected_wmo,
        ),
        Ok(())
    );
    for invalid in [vec![0], vec![9, 8], vec![9, 9]] {
        assert!(
            validate_material_filtered_wmo(
                &invalid,
                coverage.missing_wmo_policy,
                &coverage.missing_m2,
                &coverage.missing_wmo,
                &coverage.empty_wmo,
                &coverage.unselected_wmo,
            )
            .is_err()
        );
    }
    assert_eq!(
        validate_material_filtered_wmo(
            &vec![1; COVERAGE_MAX_MATERIAL_FILTERED_WMO + 1],
            coverage.missing_wmo_policy,
            &coverage.missing_m2,
            &coverage.missing_wmo,
            &coverage.empty_wmo,
            &coverage.unselected_wmo,
        )
        .unwrap_err()
        .code,
        LIMIT
    );
    for overlapping in [
        coverage.missing_m2[0],
        coverage.missing_wmo[0],
        coverage.empty_wmo[0],
        coverage.unselected_wmo[0],
    ] {
        assert!(
            validate_material_filtered_wmo(
                &[overlapping],
                coverage.missing_wmo_policy,
                &coverage.missing_m2,
                &coverage.missing_wmo,
                &coverage.empty_wmo,
                &coverage.unselected_wmo,
            )
            .is_err()
        );
    }
    assert!(
        validate_material_filtered_wmo(&[1], run::MissingWmoPolicy::Reject, &[], &[], &[], &[])
            .is_err()
    );

    let bytes = coverage.encode().unwrap();
    let resource = resources::Profile::RequestHeavyV1.id();
    let resource_at = bytes
        .windows(resource.len())
        .position(|window| window == resource.as_bytes())
        .unwrap();
    let missing_count_at = resource_at + resource.len() + 1;
    let missing_count = u32::from_le_bytes(
        bytes[missing_count_at..missing_count_at + 4]
            .try_into()
            .unwrap(),
    ) as usize;
    let empty_count_at = missing_count_at + 4 + missing_count * 4;
    let empty_count = u32::from_le_bytes(
        bytes[empty_count_at..empty_count_at + 4]
            .try_into()
            .unwrap(),
    ) as usize;
    let unselected_count_at = empty_count_at + 4 + empty_count * 4;
    let unselected_count = u32::from_le_bytes(
        bytes[unselected_count_at..unselected_count_at + 4]
            .try_into()
            .unwrap(),
    ) as usize;
    let material_count_at = unselected_count_at + 4 + unselected_count * 4;
    let material_count = u32::from_le_bytes(
        bytes[material_count_at..material_count_at + 4]
            .try_into()
            .unwrap(),
    ) as usize;
    assert_eq!(material_count, coverage.material_filtered_wmo.len());
    let first_id_at = material_count_at + 4;
    assert_eq!(
        &bytes[first_id_at..first_id_at + 4],
        &coverage.material_filtered_wmo[0].to_le_bytes()
    );
    let second_id_at = first_id_at + 4;
    let mut too_many = bytes.clone();
    too_many[material_count_at..material_count_at + 4]
        .copy_from_slice(&((COVERAGE_MAX_MATERIAL_FILTERED_WMO + 1) as u32).to_le_bytes());
    assert!(Coverage::parse_v36(&too_many).is_err());
    for invalid_id in [0, coverage.material_filtered_wmo[1]] {
        let mut mutated = bytes.clone();
        mutated[first_id_at..first_id_at + 4].copy_from_slice(&invalid_id.to_le_bytes());
        assert!(Coverage::parse_v36(&mutated).is_err());
    }
    let mut out_of_order = bytes.clone();
    out_of_order[second_id_at..second_id_at + 4].copy_from_slice(&1_u32.to_le_bytes());
    assert!(Coverage::parse_v36(&out_of_order).is_err());
    for overlapping in [
        coverage.missing_m2[0],
        coverage.missing_wmo[0],
        coverage.empty_wmo[0],
        coverage.unselected_wmo[0],
    ] {
        let mut mutated = bytes.clone();
        mutated[first_id_at..first_id_at + 4].copy_from_slice(&overlapping.to_le_bytes());
        assert!(Coverage::parse_v36(&mutated).is_err());
    }
}

#[test]
fn coverage_v8_enforces_bounded_sorted_nonzero_disjoint_unselected_wmo_ids() {
    let coverage = unselected_wmo_sample();
    assert_eq!(
        validate_unselected_wmo(
            &coverage.unselected_wmo,
            coverage.missing_wmo_policy,
            &coverage.missing_m2,
            &coverage.missing_wmo,
            &coverage.empty_wmo,
        ),
        Ok(())
    );
    for invalid in [vec![0], vec![9, 8], vec![9, 9]] {
        assert!(
            validate_unselected_wmo(
                &invalid,
                coverage.missing_wmo_policy,
                &coverage.missing_m2,
                &coverage.missing_wmo,
                &coverage.empty_wmo,
            )
            .is_err()
        );
    }
    assert_eq!(
        validate_unselected_wmo(
            &vec![1; COVERAGE_MAX_UNSELECTED_WMO + 1],
            coverage.missing_wmo_policy,
            &coverage.missing_m2,
            &coverage.missing_wmo,
            &coverage.empty_wmo,
        )
        .unwrap_err()
        .code,
        LIMIT
    );
    for overlapping in [
        coverage.missing_m2[0],
        coverage.missing_wmo[0],
        coverage.empty_wmo[0],
    ] {
        assert!(
            validate_unselected_wmo(
                &[overlapping],
                coverage.missing_wmo_policy,
                &coverage.missing_m2,
                &coverage.missing_wmo,
                &coverage.empty_wmo,
            )
            .is_err()
        );
    }
    assert!(validate_unselected_wmo(&[1], run::MissingWmoPolicy::Reject, &[], &[], &[],).is_err());

    let bytes = coverage.encode().unwrap();
    let resource = resources::Profile::RequestHeavyV1.id();
    let resource_at = bytes
        .windows(resource.len())
        .position(|window| window == resource.as_bytes())
        .unwrap();
    let missing_count_at = resource_at + resource.len() + 1;
    let missing_count = u32::from_le_bytes(
        bytes[missing_count_at..missing_count_at + 4]
            .try_into()
            .unwrap(),
    ) as usize;
    let empty_count_at = missing_count_at + 4 + missing_count * 4;
    let empty_count = u32::from_le_bytes(
        bytes[empty_count_at..empty_count_at + 4]
            .try_into()
            .unwrap(),
    ) as usize;
    let unselected_count_at = empty_count_at + 4 + empty_count * 4;
    let unselected_count = u32::from_le_bytes(
        bytes[unselected_count_at..unselected_count_at + 4]
            .try_into()
            .unwrap(),
    ) as usize;
    assert_eq!(unselected_count, coverage.unselected_wmo.len());
    let first_id_at = unselected_count_at + 4;
    assert_eq!(
        &bytes[first_id_at..first_id_at + 4],
        &coverage.unselected_wmo[0].to_le_bytes()
    );
    let second_id_at = first_id_at + 4;
    let mut too_many = bytes.clone();
    too_many[unselected_count_at..unselected_count_at + 4]
        .copy_from_slice(&((COVERAGE_MAX_UNSELECTED_WMO + 1) as u32).to_le_bytes());
    assert!(Coverage::parse_v35(&too_many).is_err());
    for invalid_id in [0, coverage.unselected_wmo[1]] {
        let mut mutated = bytes.clone();
        mutated[first_id_at..first_id_at + 4].copy_from_slice(&invalid_id.to_le_bytes());
        assert!(Coverage::parse_v35(&mutated).is_err());
    }
    let mut out_of_order = bytes.clone();
    out_of_order[second_id_at..second_id_at + 4].copy_from_slice(&1_u32.to_le_bytes());
    assert!(Coverage::parse_v35(&out_of_order).is_err());
    for overlapping in [
        coverage.missing_m2[0],
        coverage.missing_wmo[0],
        coverage.empty_wmo[0],
    ] {
        let mut mutated = bytes.clone();
        mutated[first_id_at..first_id_at + 4].copy_from_slice(&overlapping.to_le_bytes());
        assert!(Coverage::parse_v35(&mutated).is_err());
    }
}

#[test]
fn coverage_v5_rejects_crossed_resource_profiles() {
    let bytes = request_capacity_sample().encode().unwrap();
    for profile in [
        resources::Profile::BoundedV1,
        resources::Profile::DiskWorldV1,
    ] {
        let mutated = replace_text(
            &bytes,
            resources::Profile::RequestHeavyV1.id(),
            profile.id(),
        );
        assert!(Coverage::parse_v32(&mutated).is_err());
    }
    assert_eq!(
        coverage_version(GeometryPolicy::new(
            MapSelection::All,
            run::MissingWdtPolicy::DeclareRootAbsent,
            run::MissingModelPolicy::DeclareRootAbsentWmoDoodadM2,
            resources::Profile::BoundedV1,
        ))
        .unwrap(),
        MODEL_ROOT_ABSENT_COVERAGE_VERSION
    );
    for (selection, resource_profile) in [
        (MapSelection::All, resources::Profile::DiskWorldV1),
        (
            MapSelection::MapAndAncestors(2),
            resources::Profile::RequestHeavyV1,
        ),
    ] {
        let policy = GeometryPolicy::new(
            selection,
            run::MissingWdtPolicy::DeclareRootAbsent,
            run::MissingModelPolicy::DeclareRootAbsentWmoDoodadM2,
            resource_profile,
        );
        assert!(coverage_version(policy).is_err());
        assert!(coverage_path(policy).is_err());
    }
}

#[test]
fn coverage_v4_rejects_zero_duplicate_unordered_and_false_model_policy() {
    let bytes = model_root_absent_sample().encode().unwrap();
    let at = after_context(&bytes);
    let profile = crate::plan::MODEL_ROOT_ABSENT_GEOMETRY_COVERAGE_PROFILE;
    let length = u32::from_le_bytes(bytes[at..at + 4].try_into().unwrap()) as usize;
    assert_eq!(&bytes[at + 4..at + 4 + length], profile.as_bytes());

    let first = 0x1122_3344_u32.to_le_bytes();
    let first_at = bytes
        .windows(first.len())
        .position(|window| window == first)
        .unwrap();
    assert_eq!(bytes[first_at - 5], 1);
    assert_eq!(
        u32::from_le_bytes(bytes[first_at - 4..first_at].try_into().unwrap()),
        2
    );
    let second_at = first_at + 4;

    let mut zero = bytes.clone();
    zero[first_at..first_at + 4].copy_from_slice(&0_u32.to_le_bytes());
    assert!(Coverage::parse_v31(&zero).is_err());

    let mut duplicate = bytes.clone();
    duplicate[second_at..second_at + 4].copy_from_slice(&first);
    assert!(Coverage::parse_v31(&duplicate).is_err());

    let mut unordered = bytes.clone();
    unordered[second_at..second_at + 4].copy_from_slice(&1_u32.to_le_bytes());
    assert!(Coverage::parse_v31(&unordered).is_err());

    for invalid in [0, 2] {
        let mut mutated = bytes.clone();
        mutated[first_at - 5] = invalid;
        assert!(Coverage::parse_v31(&mutated).is_err());
    }

    let too_many: Vec<u32> = (1..=COVERAGE_MAX_MISSING_M2 as u32 + 1).collect();
    assert_eq!(
        validate_missing_m2(
            &too_many,
            run::MissingModelPolicy::DeclareRootAbsentWmoDoodadM2
        )
        .unwrap_err()
        .code,
        LIMIT
    );
}

#[test]
fn coverage_versions_reject_every_truncation_and_any_suffix() {
    for (bytes, parse) in [
        (
            sample().encode().unwrap(),
            Coverage::parse as fn(&[u8]) -> Result<Coverage, Error>,
        ),
        (
            available_sample().encode().unwrap(),
            Coverage::parse_v29 as fn(&[u8]) -> Result<Coverage, Error>,
        ),
        (
            root_absent_sample().encode().unwrap(),
            Coverage::parse_v30 as fn(&[u8]) -> Result<Coverage, Error>,
        ),
        (
            model_root_absent_sample().encode().unwrap(),
            Coverage::parse_v31 as fn(&[u8]) -> Result<Coverage, Error>,
        ),
        (
            request_capacity_sample().encode().unwrap(),
            Coverage::parse_v32 as fn(&[u8]) -> Result<Coverage, Error>,
        ),
        (
            vacated_wmo_sample().encode().unwrap(),
            Coverage::parse_v33 as fn(&[u8]) -> Result<Coverage, Error>,
        ),
        (
            empty_wmo_sample().encode().unwrap(),
            Coverage::parse_v34 as fn(&[u8]) -> Result<Coverage, Error>,
        ),
        (
            unselected_wmo_sample().encode().unwrap(),
            Coverage::parse_v35 as fn(&[u8]) -> Result<Coverage, Error>,
        ),
        (
            material_filtered_wmo_sample().encode().unwrap(),
            Coverage::parse_v36 as fn(&[u8]) -> Result<Coverage, Error>,
        ),
        (
            dependency_heavy_material_filtered_wmo_sample()
                .encode()
                .unwrap(),
            Coverage::parse_v37 as fn(&[u8]) -> Result<Coverage, Error>,
        ),
        (
            source_heavy_material_filtered_wmo_sample()
                .encode()
                .unwrap(),
            Coverage::parse_v38 as fn(&[u8]) -> Result<Coverage, Error>,
        ),
        (
            read_heavy_material_filtered_wmo_sample().encode().unwrap(),
            Coverage::parse_v39 as fn(&[u8]) -> Result<Coverage, Error>,
        ),
        (
            model_heavy_material_filtered_wmo_sample().encode().unwrap(),
            Coverage::parse_v40 as fn(&[u8]) -> Result<Coverage, Error>,
        ),
        (
            source_expanded_material_filtered_wmo_sample()
                .encode()
                .unwrap(),
            Coverage::parse_v41 as fn(&[u8]) -> Result<Coverage, Error>,
        ),
        (
            model_output_expanded_material_filtered_wmo_sample()
                .encode()
                .unwrap(),
            Coverage::parse_v42 as fn(&[u8]) -> Result<Coverage, Error>,
        ),
        (
            model_bytes_expanded_material_filtered_wmo_sample()
                .encode()
                .unwrap(),
            Coverage::parse_v43 as fn(&[u8]) -> Result<Coverage, Error>,
        ),
        (
            source_expanded_v2_material_filtered_wmo_sample()
                .encode()
                .unwrap(),
            Coverage::parse_v44 as fn(&[u8]) -> Result<Coverage, Error>,
        ),
        (
            placement_expanded_material_filtered_wmo_sample()
                .encode()
                .unwrap(),
            Coverage::parse_v45 as fn(&[u8]) -> Result<Coverage, Error>,
        ),
        (
            assembly_expanded_material_filtered_wmo_sample()
                .encode()
                .unwrap(),
            Coverage::parse_v46 as fn(&[u8]) -> Result<Coverage, Error>,
        ),
        (
            gameobject_expanded_material_filtered_wmo_sample()
                .encode()
                .unwrap(),
            Coverage::parse_v47 as fn(&[u8]) -> Result<Coverage, Error>,
        ),
    ] {
        for end in 0..bytes.len() {
            assert!(
                parse(&bytes[..end]).is_err(),
                "accepted truncation at {end}"
            );
        }
        for suffix in [vec![0], vec![1], vec![0; 32]] {
            let mut extended = bytes.clone();
            extended.extend_from_slice(&suffix);
            assert!(parse(&extended).is_err());
        }
    }
}

#[test]
fn omitted_maps_reject_tree_proof_and_tile_artifacts_but_not_present_map_artifacts() {
    let coverage = root_absent_sample();
    let (tree, proof) = map_paths(2);
    for path in [
        tree,
        proof,
        tile_path(2, 0, 0, true),
        tile_path(2, 0, 0, false),
    ] {
        let mut inventory = BTreeMap::new();
        inventory.insert(
            path.clone(),
            ArtifactFact {
                path,
                bytes: 0,
                sha: [0; 32],
            },
        );
        assert!(validate_omitted_artifacts(&coverage.maps, &inventory).is_err());
    }

    let (tree, _) = map_paths(1);
    let mut inventory = BTreeMap::new();
    inventory.insert(
        tree.clone(),
        ArtifactFact {
            path: tree,
            bytes: 0,
            sha: [0; 32],
        },
    );
    assert!(validate_omitted_artifacts(&coverage.maps, &inventory).is_ok());
}

#[test]
fn empty_and_missing_wmo_reject_omitted_models_vmos_and_placements() {
    for is_missing in [true, false] {
        let id = if is_missing { 0x1122_3344 } else { 0x5566_7788 };
        let omitted = m2_file_name(id);
        let missing_wmo = if is_missing { vec![id] } else { Vec::new() };
        let empty_wmo = if is_missing { Vec::new() } else { vec![id] };
        let artifact = || {
            let path = format!("vmaps/{omitted}.vmo");
            (
                path.clone(),
                ArtifactFact {
                    path,
                    bytes: 1,
                    sha: [1; 32],
                },
            )
        };

        let mut used_models = BTreeMap::from([("present.xxx".to_owned(), ())]);
        let mut artifacts = BTreeMap::new();
        assert!(validate_wmo_omission_artifacts(&[id], &used_models, &artifacts).is_ok());

        used_models.insert(omitted.clone(), ());
        assert!(validate_wmo_omission_artifacts(&[id], &used_models, &artifacts).is_err());
        used_models.remove(&omitted);
        let (path, fact) = artifact();
        artifacts.insert(path, fact);
        assert!(validate_wmo_omission_artifacts(&[id], &used_models, &artifacts).is_err());

        assert!(is_omitted_wmo_name(
            &omitted,
            &[],
            &missing_wmo,
            &empty_wmo,
            &[],
            &[],
            false,
        ));
        assert!(!is_omitted_wmo_name(
            &format!("{omitted}-extra"),
            &[],
            &missing_wmo,
            &empty_wmo,
            &[],
            &[],
            false,
        ));
    }
}

#[test]
fn missing_wmo_doodads_reject_only_their_used_models_and_vmos() {
    let omitted = m2_file_name(0x1122_3344);
    let present = m2_file_name(0x5566_7788);
    let artifact = |name: &str| {
        let path = format!("vmaps/{name}.vmo");
        (
            path.clone(),
            ArtifactFact {
                path,
                bytes: 1,
                sha: [1; 32],
            },
        )
    };
    let mut used_models = BTreeMap::from([("wmo-parent".to_owned(), ()), (present.clone(), ())]);
    let mut artifacts = BTreeMap::from([artifact("wmo-parent"), artifact(&present)]);
    assert!(validate_missing_model_artifacts(&[0x1122_3344], &used_models, &artifacts).is_ok());
    assert!(validate_wmo_omission_artifacts(&[0x1122_3344], &used_models, &artifacts).is_ok());

    used_models.insert(omitted.clone(), ());
    assert!(validate_missing_model_artifacts(&[0x1122_3344], &used_models, &artifacts).is_err());
    assert!(validate_wmo_omission_artifacts(&[0x1122_3344], &used_models, &artifacts).is_err());
    used_models.remove(&omitted);
    let (omitted_path, omitted_fact) = artifact(&omitted);
    artifacts.insert(omitted_path, omitted_fact);
    assert!(validate_missing_model_artifacts(&[0x1122_3344], &used_models, &artifacts).is_err());
    assert!(validate_wmo_omission_artifacts(&[0x1122_3344], &used_models, &artifacts).is_err());

    let prefix_artifacts = BTreeMap::from([artifact("FILE11223344-extra.xxx")]);
    assert!(
        validate_wmo_omission_artifacts(&[0x1122_3344], &used_models, &prefix_artifacts).is_ok()
    );
    let prefix_used = BTreeMap::from([("FILE11223344-extra.xxx".to_owned(), ())]);
    let no_artifacts = BTreeMap::new();
    assert!(validate_wmo_omission_artifacts(&[0x1122_3344], &prefix_used, &no_artifacts).is_ok());
    assert!(is_omitted_wmo_name(
        &omitted,
        &[],
        &[0x1122_3344],
        &[],
        &[],
        &[],
        false
    ));
    assert!(is_omitted_wmo_name(
        &omitted,
        &[],
        &[],
        &[0x1122_3344],
        &[],
        &[],
        false
    ));
    assert!(is_omitted_wmo_name(
        &omitted,
        &[0x1122_3344],
        &[],
        &[],
        &[],
        &[],
        true
    ));
    assert!(is_omitted_wmo_name(
        &omitted,
        &[],
        &[],
        &[],
        &[0x1122_3344],
        &[],
        true
    ));
    assert!(!is_omitted_wmo_name(
        "FILE11223344-extra.xxx",
        &[0x1122_3344],
        &[],
        &[],
        &[],
        &[],
        true,
    ));

    let unselected_name = m2_file_name(0x0102_0304);
    let unselected_used = BTreeMap::from([(unselected_name.clone(), ())]);
    let unselected_artifacts = BTreeMap::from([artifact(&unselected_name)]);
    assert!(validate_wmo_omission_artifacts(&[0x0102_0304], &unselected_used, &artifacts).is_err());
    assert!(
        validate_wmo_omission_artifacts(&[0x0102_0304], &used_models, &unselected_artifacts)
            .is_err()
    );
}

#[test]
fn v42_and_v43_expand_local_model_count_and_use_model_diagnostics() {
    for profile in [
        resources::Profile::ModelOutputExpandedV1,
        resources::Profile::ModelBytesExpandedV1,
    ] {
        assert_eq!(
            super::local_model_limit(profile),
            (65_536, "product_selected_geometry_model_count_limit")
        );
    }
    assert_eq!(
        super::MODEL_BYTES_EXPANDED_MATERIAL_FILTERED_WMO_COVERAGE_PATH,
        "vmaps/available-model-bytes-expanded-material-filtered-wmo-geometry-coverage-v16.bin"
    );
    assert_eq!(
        super::MODEL_BYTES_EXPANDED_MATERIAL_FILTERED_WMO_CODEC_PROFILE,
        "linux70450-model-bytes-expanded-material-filtered-wmo-v1"
    );
    assert_eq!(
        super::MODEL_BYTES_EXPANDED_MATERIAL_FILTERED_WMO_COMPLETION_PROFILE,
        "linux70450-available-model-bytes-expanded-material-filtered-wmo-collision-v1"
    );
    assert_eq!(
        super::MODEL_BYTES_EXPANDED_MATERIAL_FILTERED_WMO_COVERAGE_VERSION,
        16
    );
    assert_eq!(
        super::coverage_version(super::GeometryPolicy::new_with_material_filtered_wmo(
            MapSelection::All,
            run::MissingWdtPolicy::DeclareRootAbsent,
            run::MissingModelPolicy::DeclareRootAbsentWmoDoodadM2,
            resources::Profile::ModelBytesExpandedV1,
        )),
        Ok(16)
    );
    for profile in [
        resources::Profile::BoundedV1,
        resources::Profile::DiskWorldV1,
        resources::Profile::RequestHeavyV1,
        resources::Profile::DependencyHeavyV1,
        resources::Profile::SourceHeavyV1,
        resources::Profile::ReadHeavyV1,
        resources::Profile::ModelHeavyV1,
        resources::Profile::SourceExpandedV1,
    ] {
        assert_eq!(
            super::local_model_limit(profile),
            (4096, "product_selected_geometry_limit"),
            "{profile:?}"
        );
    }
}

#[cfg(target_os = "linux")]
#[test]
fn v42_stages_model_4097_while_v41_rejects_it_at_the_real_sink_boundary() {
    use forever_casc::{
        Locale,
        collision::resources::Profile,
        local_store::LocalStore,
        package::{Component, Context, Limits as PackageLimits, Run},
    };
    use std::{
        fs,
        os::unix::fs::PermissionsExt,
        sync::atomic::{AtomicU64, Ordering},
    };

    const MODEL_COUNT: usize = 4097;
    static NEXT: AtomicU64 = AtomicU64::new(0);
    struct Fixture(std::path::PathBuf);
    impl Drop for Fixture {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }
    let root_dir = std::env::var_os("HOME")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(std::env::temp_dir)
        .join(format!(
            ".local/share/rce-v42-staged-model-boundary-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
    fs::create_dir_all(root_dir.parent().unwrap()).unwrap();
    fs::create_dir(&root_dir).unwrap();
    fs::set_permissions(&root_dir, fs::Permissions::from_mode(0o700)).unwrap();
    let _fixture = Fixture(root_dir.clone());
    let client = root_dir.join("client");
    fs::create_dir(&client).unwrap();
    fs::set_permissions(&client, fs::Permissions::from_mode(0o700)).unwrap();
    let names = model_staging_fixture::write_named_model_client(&client, MODEL_COUNT);
    let mut store = LocalStore::open_db2_70450(
        &client,
        forever_casc::local_store::OpenOptions {
            locale: Locale::EsEs,
            index_layouts: [forever_casc::local_index::Layout::Contiguous; 16],
            root_layout: forever_casc::root::Layout::Header50893V2,
            limits: Default::default(),
        },
    )
    .unwrap();
    store
        .enable_names(forever_casc::root::name::Limits::default())
        .unwrap();
    let context = Context {
        build: 70450,
        locale: Locale::EsEs,
        components: Component::Db2.bit() | Component::Collision.bit(),
        source_sha256: [0x31; 32],
        plan_sha256: [0x32; 32],
        producer_sha256: [0x33; 32],
    };
    for profile in [Profile::ModelOutputExpandedV1, Profile::SourceExpandedV1] {
        let output = root_dir.join(profile.id());
        let mut run = Run::create(&output, context, &[], PackageLimits::default()).unwrap();
        let mut sink = super::Sink {
            run: &mut run,
            models: BTreeMap::new(),
            placements: BTreeMap::new(),
            model_bytes: 0,
            model_limit: super::MAX_STAGED_MODEL_BYTES,
            max_models: super::local_model_limit(profile).0,
            model_count_limit: super::local_model_limit(profile).1,
            model_bytes_limit: if profile == Profile::ModelOutputExpandedV1 {
                super::MODEL_BYTES_LIMIT
            } else {
                super::LIMIT
            },
            placement_bytes: 0,
            placement_limit: super::MAX_STAGED_PLACEMENT_BYTES,
            placement_map_limit: forever_vmap::placement::Limits::default().max_bytes,
            placement_capacity: 0,
            placement_error_codes: super::placement_error_codes(profile),
            failure: None,
        };
        let limits = profile.collector_limits();
        let mut extractor = forever_casc::collision::Extractor::new_with_resources(
            &mut store, &Lookup, limits, profile,
        )
        .unwrap();
        for name in &names {
            extractor
                .extract(forever_casc::collision::ModelRequest::M2(
                    forever_casc::collision::InputRef::LogicalName(name.as_bytes()),
                ))
                .unwrap();
        }
        let extraction = extractor.finish().unwrap();
        assert_eq!(extraction.models().len(), MODEL_COUNT);
        for (index, model) in extraction.models().iter().enumerate() {
            assert_eq!(model.status(), Status::Collision);
            let result = <super::Sink<'_> as run::Sink>::model(&mut sink, model);
            if profile == Profile::ModelOutputExpandedV1 || index < 4096 {
                assert_eq!(result, Ok(()));
            } else {
                assert_eq!(result, Err(run::SinkError::Rejected));
            }
        }
        if profile == Profile::ModelOutputExpandedV1 {
            assert_eq!(sink.models.len(), MODEL_COUNT);
            assert_eq!(sink.failure, None);
        } else {
            assert_eq!(sink.models.len(), 4096);
            assert_eq!(sink.failure.unwrap().code, LIMIT);
        }
    }
}

#[cfg(target_os = "linux")]
#[test]
fn v42_model_byte_budget_uses_the_product_diagnostic_after_conversion() {
    use forever_casc::{
        Locale,
        collision::resources::Profile,
        local_store::LocalStore,
        package::{Component, Context, Limits as PackageLimits, Run},
    };
    use std::{
        fs,
        os::unix::fs::PermissionsExt,
        sync::atomic::{AtomicU64, Ordering},
    };

    static NEXT: AtomicU64 = AtomicU64::new(0);
    struct Fixture(std::path::PathBuf);
    impl Drop for Fixture {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }
    let root_dir = std::env::var_os("HOME")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(std::env::temp_dir)
        .join(format!(
            ".local/share/rce-v42-model-byte-budget-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
    fs::create_dir_all(root_dir.parent().unwrap()).unwrap();
    fs::create_dir(&root_dir).unwrap();
    fs::set_permissions(&root_dir, fs::Permissions::from_mode(0o700)).unwrap();
    let _fixture = Fixture(root_dir.clone());
    let client = root_dir.join("client");
    fs::create_dir(&client).unwrap();
    fs::set_permissions(&client, fs::Permissions::from_mode(0o700)).unwrap();
    let names = model_staging_fixture::write_named_model_client(&client, 1);
    let mut store = LocalStore::open_db2_70450(
        &client,
        forever_casc::local_store::OpenOptions {
            locale: Locale::EsEs,
            index_layouts: [forever_casc::local_index::Layout::Contiguous; 16],
            root_layout: forever_casc::root::Layout::Header50893V2,
            limits: Default::default(),
        },
    )
    .unwrap();
    store
        .enable_names(forever_casc::root::name::Limits::default())
        .unwrap();
    let context = Context {
        build: 70450,
        locale: Locale::EsEs,
        components: Component::Db2.bit() | Component::Collision.bit(),
        source_sha256: [0x31; 32],
        plan_sha256: [0x32; 32],
        producer_sha256: [0x33; 32],
    };
    let mut run = Run::create(
        &root_dir.join("package"),
        context,
        &[],
        PackageLimits::default(),
    )
    .unwrap();
    let mut sink = super::Sink {
        run: &mut run,
        models: BTreeMap::new(),
        placements: BTreeMap::new(),
        model_bytes: 0,
        model_limit: super::MAX_STAGED_MODEL_BYTES,
        max_models: super::local_model_limit(Profile::ModelOutputExpandedV1).0,
        model_count_limit: super::MODEL_COUNT_LIMIT,
        model_bytes_limit: super::MODEL_BYTES_LIMIT,
        placement_bytes: 0,
        placement_limit: super::MAX_STAGED_PLACEMENT_BYTES,
        placement_map_limit: forever_vmap::placement::Limits::default().max_bytes,
        placement_capacity: 0,
        placement_error_codes: super::placement_error_codes(Profile::ModelOutputExpandedV1),
        failure: None,
    };
    let mut extractor = forever_casc::collision::Extractor::new_with_resources(
        &mut store,
        &Lookup,
        Profile::ModelOutputExpandedV1.collector_limits(),
        Profile::ModelOutputExpandedV1,
    )
    .unwrap();
    extractor
        .extract(forever_casc::collision::ModelRequest::M2(
            forever_casc::collision::InputRef::LogicalName(names[0].as_bytes()),
        ))
        .unwrap();
    let model = extractor.finish().unwrap();
    let converted_size = {
        let raw = RawM2::parse(
            model.models()[0].raw().unwrap(),
            forever_vmap::Limits::default(),
        )
        .unwrap();
        vmo::convert_m2(&raw, vmo::Limits::default())
            .unwrap()
            .bytes()
            .len()
    };
    sink.model_limit = converted_size - 1;
    let error = sink.stage_model(&model.models()[0]).unwrap_err();
    assert_eq!(error.code, super::MODEL_BYTES_LIMIT);
    assert_eq!(sink.failure, None);
}

#[test]
fn placement_staging_preserves_legacy_errors_and_reports_v45_v46_boundaries_separately() {
    use forever_casc::collision::resources::Profile;

    let legacy = super::placement_error_codes(Profile::SourceExpandedV2);
    let v45 = super::placement_error_codes(Profile::PlacementExpandedV1);
    let v46 = super::placement_error_codes(Profile::AssemblyExpandedV1);
    assert_eq!(legacy.aggregate, "product_selected_geometry_limit");
    assert_eq!(legacy.map, "product_selected_geometry_limit");
    assert_eq!(legacy.domain, "product_selected_geometry_domain");
    assert_eq!(legacy.domain_count, "product_selected_geometry_limit");
    assert_eq!(
        v45.aggregate,
        "product_selected_geometry_placement_bytes_limit"
    );
    assert_eq!(v45.map, "product_selected_geometry_placement_map_limit");
    assert_eq!(v45.domain, "product_selected_geometry_domain_limit");
    assert_eq!(v45.domain_count, "product_selected_geometry_domain_limit");
    assert_eq!(
        v46.aggregate,
        "product_selected_geometry_placement_bytes_limit"
    );
    assert_eq!(v46.map, "product_selected_geometry_placement_map_limit");
    assert_eq!(v46.domain, "product_selected_geometry_domain_limit");
    assert_eq!(v46.domain_count, "product_selected_geometry_domain_limit");

    let plan_limit = 512 * 1024 * 1024;
    assert_eq!(
        super::staged_placement_limit(Profile::SourceExpandedV2, plan_limit).unwrap(),
        128 * 1024 * 1024
    );
    assert_eq!(
        super::staged_placement_limit(Profile::PlacementExpandedV1, plan_limit).unwrap(),
        256 * 1024 * 1024
    );
    assert_eq!(
        super::staged_placement_limit(Profile::AssemblyExpandedV1, plan_limit).unwrap(),
        256 * 1024 * 1024
    );
    assert_eq!(
        super::staged_placement_limit(Profile::AssemblyExpandedV1, 192 * 1024 * 1024).unwrap(),
        192 * 1024 * 1024
    );
    assert_eq!(
        super::staged_placement_limit(Profile::PlacementExpandedV1, 192 * 1024 * 1024).unwrap(),
        192 * 1024 * 1024
    );

    let per_map_limit = forever_vmap::placement::Limits::default().max_bytes;
    assert_eq!(per_map_limit, 64 * 1024 * 1024);
    let total_limit =
        super::staged_placement_limit(Profile::PlacementExpandedV1, plan_limit).unwrap();
    assert_eq!(
        super::next_placement_lengths(
            total_limit - per_map_limit,
            0,
            per_map_limit,
            total_limit,
            per_map_limit,
            v45,
        )
        .unwrap(),
        (total_limit, per_map_limit)
    );
    let aggregate_error =
        super::next_placement_lengths(total_limit, 0, 1, total_limit, per_map_limit, v45)
            .unwrap_err();
    assert_eq!(aggregate_error.code, super::PLACEMENT_BYTES_LIMIT);
    assert_eq!(aggregate_error.component, None);
    let map_error =
        super::next_placement_lengths(0, per_map_limit, 1, total_limit, per_map_limit, v45)
            .unwrap_err();
    assert_eq!(map_error.code, super::PLACEMENT_MAP_LIMIT);
    assert_eq!(map_error.component, None);
    let legacy_error = super::next_placement_lengths(
        128 * 1024 * 1024,
        0,
        1,
        128 * 1024 * 1024,
        per_map_limit,
        legacy,
    )
    .unwrap_err();
    assert_eq!(legacy_error.code, super::LIMIT);
    assert_eq!(legacy_error.component, None);
    assert_eq!(
        super::next_placement_lengths(0, per_map_limit, 0, total_limit, per_map_limit, v45)
            .unwrap(),
        (0, per_map_limit)
    );
}

#[cfg(target_os = "linux")]
#[test]
fn stage_placements_returns_distinct_v45_errors_and_keeps_legacy_aggregate_code() {
    use forever_casc::{
        collision::{resources::Profile, run::Kind},
        package::{Component, Context, Limits as PackageLimits, Run},
    };
    use std::{
        fs,
        os::unix::fs::PermissionsExt,
        sync::atomic::{AtomicU64, Ordering},
    };

    static NEXT: AtomicU64 = AtomicU64::new(0);
    let root = std::env::var_os("HOME")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(std::env::temp_dir)
        .join(format!(
            ".local/share/rce-v45-stage-placements-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
    fs::create_dir_all(root.parent().unwrap()).unwrap();
    fs::create_dir(&root).unwrap();
    fs::set_permissions(&root, fs::Permissions::from_mode(0o700)).unwrap();
    struct Cleanup(std::path::PathBuf);
    impl Drop for Cleanup {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }
    let _cleanup = Cleanup(root.clone());
    let context = Context {
        build: 70450,
        locale: Locale::EsEs,
        components: Component::Db2.bit() | Component::Collision.bit(),
        source_sha256: [0x31; 32],
        plan_sha256: [0x32; 32],
        producer_sha256: [0x33; 32],
    };
    let mut run = Run::create(
        &root.join("package"),
        context,
        &[],
        PackageLimits::default(),
    )
    .unwrap();
    let mut sink = super::Sink {
        run: &mut run,
        models: BTreeMap::new(),
        placements: BTreeMap::new(),
        model_bytes: 0,
        model_limit: super::MAX_STAGED_MODEL_BYTES,
        max_models: 65_536,
        model_count_limit: super::MODEL_COUNT_LIMIT,
        model_bytes_limit: super::MODEL_BYTES_LIMIT,
        placement_bytes: 0,
        placement_limit: 4,
        placement_map_limit: 8,
        placement_capacity: 0,
        placement_error_codes: super::placement_error_codes(Profile::PlacementExpandedV1),
        failure: None,
    };
    let placement_context = forever_casc::collision::run::Context {
        map_id: 1,
        original_map_id: 1,
        kind: Kind::Global,
        replayed: false,
    };
    sink.stage_placements(placement_context, &[0; 4]).unwrap();
    let aggregate =
        <super::Sink<'_> as run::Sink>::placements(&mut sink, placement_context, &[0; 1])
            .unwrap_err();
    assert_eq!(aggregate, run::SinkError::Rejected);
    assert_eq!(
        sink.failure.as_ref().unwrap().code,
        super::PLACEMENT_BYTES_LIMIT
    );
    assert_eq!(sink.placements.get(&1).unwrap(), &[0; 4]);
    assert_eq!(sink.placement_bytes, 4);
    assert!(sink.placement_capacity >= 4);

    sink.placement_limit = 16;
    sink.placement_map_limit = 4;
    let per_map = <super::Sink<'_> as run::Sink>::placements(&mut sink, placement_context, &[0; 1])
        .unwrap_err();
    assert_eq!(per_map, run::SinkError::Rejected);
    assert_eq!(
        sink.failure.as_ref().unwrap().code,
        super::PLACEMENT_BYTES_LIMIT
    );
    assert_eq!(sink.placements.get(&1).unwrap(), &[0; 4]);
    assert_eq!(sink.placement_bytes, 4);

    sink.failure = None;
    sink.placement_limit = usize::MAX;
    sink.placement_map_limit = usize::MAX;
    sink.placement_capacity = usize::MAX;
    let overflow_context = forever_casc::collision::run::Context {
        map_id: 2,
        ..placement_context
    };
    let overflow = <super::Sink<'_> as run::Sink>::placements(&mut sink, overflow_context, &[0; 1])
        .unwrap_err();
    assert_eq!(overflow, run::SinkError::Rejected);
    assert_eq!(
        sink.failure.as_ref().unwrap().code,
        super::PLACEMENT_BYTES_LIMIT
    );
    assert!(sink.placements.get(&2).unwrap().is_empty());
    assert_eq!(sink.placement_bytes, 4);

    let v45 = super::placement_error_codes(Profile::PlacementExpandedV1);
    assert_eq!(
        super::next_placement_lengths(usize::MAX, 0, 1, usize::MAX, 8, v45)
            .unwrap_err()
            .code,
        super::PLACEMENT_BYTES_LIMIT
    );
    assert_eq!(
        super::next_placement_lengths(0, usize::MAX, 1, usize::MAX, usize::MAX, v45)
            .unwrap_err()
            .code,
        super::PLACEMENT_MAP_LIMIT
    );

    sink.placements.clear();
    sink.placement_bytes = 0;
    sink.placement_capacity = 0;
    sink.placement_limit = 0;
    sink.placement_map_limit = 8;
    sink.placement_error_codes = super::placement_error_codes(Profile::SourceExpandedV2);
    let legacy = sink
        .stage_placements(placement_context, &[0; 1])
        .unwrap_err();
    assert_eq!(legacy.code, super::LIMIT);
    assert_eq!(legacy.component, None);
}

#[test]
fn placement_domain_limit_is_profile_specific_and_directly_validated() {
    use forever_casc::collision::resources::Profile;

    let present_ids = [1];
    let v45 = super::placement_error_codes(Profile::PlacementExpandedV1);
    let legacy = super::placement_error_codes(Profile::SourceExpandedV2);
    assert!(super::validate_placement_domain([1], &present_ids, v45.domain).is_ok());
    let outside = super::validate_placement_domain([3], &present_ids, v45.domain).unwrap_err();
    assert_eq!(outside.code, "product_selected_geometry_domain_limit");
    assert_eq!(outside.component, None);
    let root_absent = super::validate_placement_domain([2], &present_ids, v45.domain).unwrap_err();
    assert_eq!(root_absent.code, "product_selected_geometry_domain_limit");
    assert_eq!(root_absent.component, None);
    let legacy_outside =
        super::validate_placement_domain([3], &present_ids, legacy.domain).unwrap_err();
    assert_eq!(legacy_outside.code, "product_selected_geometry_domain");
    assert_eq!(legacy_outside.component, None);
    let empty_domain =
        super::validate_domain_count(MapSelection::MapAndAncestors(1), 0, v45.domain).unwrap_err();
    assert_eq!(empty_domain.code, "product_selected_geometry_domain_limit");
    assert_eq!(empty_domain.component, None);
}

#[test]
fn geometry_limits_v46_matches_v45_except_for_assembled_domain_guard() {
    let budget = DiskBudget {
        raw_bytes: 32,
        wdt_bytes: 64,
        cache_bytes: 128,
        placement_bytes: 256,
        vmo_bytes: 512,
        map_bytes: 512 * 1024 * 1024,
        output_bytes: 2048,
    };
    let placement_expanded =
        geometry_limits(budget, resources::Profile::PlacementExpandedV1).unwrap();
    let assembly_expanded =
        geometry_limits(budget, resources::Profile::AssemblyExpandedV1).unwrap();

    assert_eq!(placement_expanded.world.run.placements.max_models, 65_536);
    assert_eq!(assembly_expanded.world.run.placements.max_models, 65_536);
    assert_eq!(assembly_expanded, placement_expanded);
    assert_eq!(
        super::assembled_domain_limit(resources::Profile::PlacementExpandedV1, budget.map_bytes),
        128 * 1024 * 1024
    );
    assert_eq!(
        super::assembled_domain_limit(resources::Profile::AssemblyExpandedV1, budget.map_bytes),
        256 * 1024 * 1024
    );
    assert_ne!(
        super::assembled_domain_error(resources::Profile::PlacementExpandedV1),
        super::assembled_domain_error(resources::Profile::AssemblyExpandedV1)
    );
}

#[test]
fn geometry_limits_install_model_heavy_placement_capacity_only() {
    let budget = DiskBudget {
        raw_bytes: 32,
        wdt_bytes: 64,
        cache_bytes: 128,
        placement_bytes: 256,
        vmo_bytes: 512,
        map_bytes: 1024,
        output_bytes: 2048,
    };
    let bounded = geometry_limits(budget, resources::Profile::BoundedV1).unwrap();
    let request_heavy = geometry_limits(budget, resources::Profile::RequestHeavyV1).unwrap();
    let source_heavy = geometry_limits(budget, resources::Profile::SourceHeavyV1).unwrap();
    let read_heavy = geometry_limits(budget, resources::Profile::ReadHeavyV1).unwrap();
    let source_expanded_small =
        geometry_limits(budget, resources::Profile::SourceExpandedV1).unwrap();
    assert_eq!(bounded.startup.collision.max_requests, 4096);
    assert_eq!(request_heavy.startup.collision.max_requests, 16_000_000);
    assert_eq!(
        request_heavy.startup.max_candidate_requests,
        bounded.startup.max_candidate_requests
    );
    assert_eq!(
        request_heavy.startup.collision.max_source_bytes,
        bounded.startup.collision.max_source_bytes
    );
    assert_eq!(
        request_heavy.startup.collision.max_raw_bytes,
        bounded.startup.collision.max_raw_bytes
    );
    assert_eq!(source_heavy.startup.collision.max_source_bytes, 32);
    assert_eq!(source_heavy.startup.collision.max_raw_bytes, 32);
    assert_eq!(source_expanded_small.startup.collision.max_source_bytes, 32);
    assert_eq!(source_expanded_small.startup.collision.max_raw_bytes, 32);
    assert_eq!(
        source_expanded_small.world.run.placements.max_models,
        65_536
    );
    assert_eq!(source_heavy.startup.collision.max_requests, 16_000_000);
    assert_eq!(
        source_heavy.startup.collision.max_dependency_requests,
        16_000_000
    );
    assert_eq!(request_heavy.world.run.max_wdt_memory_bytes, 64);
    assert_eq!(request_heavy.world.run.max_cache_memory_bytes, 128);
    assert_eq!(request_heavy.world.run.max_output_bytes, 288);

    let model_budget = DiskBudget {
        raw_bytes: 2 * 1024 * 1024 * 1024,
        ..budget
    };
    let model_heavy = geometry_limits(model_budget, resources::Profile::ModelHeavyV1).unwrap();
    let model_heavy_v39 = geometry_limits(model_budget, resources::Profile::ReadHeavyV1).unwrap();
    let source_limits = geometry_limits(model_budget, resources::Profile::SourceHeavyV1).unwrap();
    let source_expanded_limits =
        geometry_limits(model_budget, resources::Profile::SourceExpandedV1).unwrap();
    let model_output_expanded_limits =
        geometry_limits(model_budget, resources::Profile::ModelOutputExpandedV1).unwrap();
    let model_bytes_expanded_limits =
        geometry_limits(model_budget, resources::Profile::ModelBytesExpandedV1).unwrap();
    assert_eq!(model_heavy.world.run.placements.max_models, 65_536);
    assert_eq!(
        source_expanded_limits.world.run.placements.max_models,
        65_536
    );
    assert_eq!(
        model_output_expanded_limits.world.run.placements.max_models,
        65_536
    );
    assert_eq!(
        model_bytes_expanded_limits.world.run.placements.max_models,
        65_536
    );
    for (version, profile, expected) in [
        ("v43", resources::Profile::ModelBytesExpandedV1, 65_536),
        ("v42", resources::Profile::ModelOutputExpandedV1, 65_536),
        ("v41", resources::Profile::SourceExpandedV1, 4096),
        ("v40", resources::Profile::ModelHeavyV1, 4096),
    ] {
        assert_eq!(super::local_model_limit(profile).0, expected, "{version}");
    }
    assert_eq!(
        model_bytes_expanded_limits.startup.collision.max_models,
        65_536
    );
    assert_eq!(
        model_bytes_expanded_limits
            .startup
            .collision
            .max_source_bytes,
        1024 * 1024 * 1024
    );
    assert_eq!(
        model_bytes_expanded_limits.startup.collision.max_raw_bytes,
        256 * 1024 * 1024
    );
    assert_eq!(
        model_bytes_expanded_limits.startup.collision.max_reads,
        1_000_000
    );
    assert_eq!(
        model_bytes_expanded_limits.world.run.max_output_bytes,
        model_budget.raw_bytes + model_budget.placement_bytes
    );
    assert_eq!(
        source_expanded_limits.startup.collision.max_source_bytes,
        1024 * 1024 * 1024
    );
    assert_eq!(
        source_expanded_limits.startup.collision.max_raw_bytes,
        256 * 1024 * 1024
    );
    assert_eq!(
        source_expanded_limits.startup.collision.max_reads,
        1_000_000
    );
    assert_eq!(model_heavy_v39.world.run.placements.max_models, 4_096);
    assert_eq!(read_heavy.world.run.placements.max_models, 4_096);
    assert_eq!(source_limits.world.run.placements.max_models, 4_096);
    assert_eq!(
        model_heavy.startup.collision.max_source_bytes,
        512 * 1024 * 1024
    );
    assert_eq!(model_heavy.startup.collision.max_reads, 1_000_000);
    assert_eq!(
        model_heavy.startup.collision.max_raw_bytes,
        256 * 1024 * 1024
    );
    assert_eq!(model_heavy.startup.collision.max_requests, 16_000_000);
    assert_eq!(
        model_heavy.startup.collision.max_dependency_requests,
        16_000_000
    );
    assert_eq!(
        source_limits.startup.collision.max_source_bytes,
        512 * 1024 * 1024
    );
    assert_eq!(
        source_limits.startup.collision.max_raw_bytes,
        256 * 1024 * 1024
    );
    assert_eq!(
        model_heavy.world.run.max_wdt_memory_bytes,
        bounded.world.run.max_wdt_memory_bytes
    );
    assert_eq!(
        model_heavy.world.run.max_cache_memory_bytes,
        bounded.world.run.max_cache_memory_bytes
    );
    assert_eq!(
        model_heavy.world.run.max_output_bytes,
        model_budget.raw_bytes + 256
    );
}

#[test]
fn v28_package_limits_add_db2_bytes_and_bound_total_files() {
    let budget = DiskBudget {
        raw_bytes: 32,
        wdt_bytes: 64,
        cache_bytes: 128,
        placement_bytes: 256,
        vmo_bytes: 512,
        map_bytes: 1024,
        output_bytes: 2048,
    };
    let limits = package_limits_v28_with_extra_files(budget, 0);
    let defaults = Limits::default();
    assert_eq!(
        limits.total_bytes,
        budget.output_bytes + super::super::DB2_TOTAL_BYTES
    );
    assert_eq!(limits.files, PACKAGE_MAX_FILES);
    assert_eq!(PACKAGE_MAX_FILES, 65_536);
    assert_eq!(limits.directories, defaults.directories);
    assert_eq!(limits.file_bytes, defaults.file_bytes);
    assert_eq!(limits.metadata_bytes, defaults.metadata_bytes);
    assert_eq!(limits.manifest_bytes, defaults.manifest_bytes);
}

#[test]
fn coverage_codec_rejects_bad_policy_facts_and_noncanonical_paths() {
    let bytes = sample().encode().unwrap();
    let mut bad_facts = bytes.clone();
    // Magic(8), version(4), context length/context and the v1 map ID(4) precede
    // the plan/source/domain/policy strings and declared source facts.
    let mut at = 8 + 4;
    let context_bytes = u32::from_le_bytes(bad_facts[at..at + 4].try_into().unwrap()) as usize;
    at += 4 + context_bytes + 4;
    for _ in 0..5 {
        let length = u32::from_le_bytes(bad_facts[at..at + 4].try_into().unwrap()) as usize;
        at += 4 + length;
    }
    for field in 0..6 {
        let length = u32::from_le_bytes(bad_facts[at..at + 4].try_into().unwrap()) as usize;
        at += 4 + length;
        if field == 0 || field == 2 || field == 5 {
            at += 1;
        }
    }
    at += 1; // No complete global-map-coverage claim.
    at += 4; // GameObjectDisplayInfo file data ID.
    at += 8; // Startup read count.
    bad_facts[at..at + 4].copy_from_slice(&0_u32.to_le_bytes());
    assert!(Coverage::parse(&bad_facts).is_err());

    let mut bad_inventory = sample();
    bad_inventory.artifacts[0].path = "maps/0002_00_00.map".into();
    assert!(Coverage::parse(&bad_inventory.encode().unwrap()).is_err());
}

#[test]
fn coverage_policies_and_domain_limits_remain_strict() {
    let mut gameobject_request = sample();
    gameobject_request.gameobject_request_made = true;
    assert!(Coverage::parse(&gameobject_request.encode().unwrap()).is_err());

    let mut startup_reads = sample();
    startup_reads.gameobject_startup_reads = 1;
    let parsed = Coverage::parse(&startup_reads.encode().unwrap()).unwrap();
    assert_eq!(parsed.gameobject_startup_reads, 0);

    let mut gameobject_id = sample();
    gameobject_id.gameobject_file_data_id += 1;
    assert!(Coverage::parse(&gameobject_id.encode().unwrap()).is_err());

    let mut liquid_request = available_sample();
    liquid_request.liquid_object_request_made = true;
    assert!(Coverage::parse_v29(&liquid_request.encode().unwrap()).is_err());

    let bytes = sample().encode().unwrap();
    for text in [
        MODEL_SCOPE_PROFILE,
        "placement_only",
        "available-section-map-partial-scope-v1",
    ] {
        assert!(Coverage::parse(&mutate_text(&bytes, text)).is_err());
    }

    let mut invalid_map = sample();
    invalid_map.map.materialized_rows = 1;
    assert!(Coverage::parse(&invalid_map.encode().unwrap()).is_err());

    let mut duplicate_maps = sample();
    duplicate_maps.maps.push(duplicate_maps.maps[0]);
    assert!(Coverage::parse(&duplicate_maps.encode().unwrap()).is_err());

    let mut too_many_maps = sample();
    too_many_maps.maps = (0..=COVERAGE_MAX_MAPS as u32)
        .map(|id| DomainMapFact {
            id,
            parent: None,
            wdt_file_data_id: 100,
            depth: 0,
            is_parent: false,
            root_absent: false,
        })
        .collect();
    let bytes = too_many_maps.encode().unwrap();
    let error = match Coverage::parse(&bytes) {
        Ok(_) => panic!("excess map count unexpectedly passed"),
        Err(error) => error,
    };
    assert_eq!(error.code, LIMIT);
}

#[test]
fn coverage_v3_rejects_zero_wdt_and_duplicate_or_overlapping_maps() {
    let mut zero_wdt = root_absent_sample();
    zero_wdt.maps[1].wdt_file_data_id = 0;
    assert!(Coverage::parse_v30(&zero_wdt.encode().unwrap()).is_err());

    let mut duplicate = root_absent_sample();
    let omitted = duplicate.maps[1];
    duplicate.maps.insert(1, omitted);
    assert!(Coverage::parse_v30(&duplicate.encode().unwrap()).is_err());
}

#[test]
fn coverage_codec_rejects_section_ordinals_outside_the_db2_section_limit() {
    let mut coverage = sample();
    coverage.map.sections[0].ordinal = 999;
    let bytes = coverage.encode().unwrap();
    let error = match Coverage::parse(&bytes) {
        Ok(_) => panic!("out-of-range Map section ordinal unexpectedly passed"),
        Err(error) => error,
    };
    assert_eq!(error.code, SCHEMA);
}

#[test]
fn available_empty_domain_does_not_inherit_the_selected_nonempty_rule() {
    assert!(validate_domain_count(MapSelection::All, 0, LIMIT).is_ok());
    assert_eq!(
        validate_domain_count(MapSelection::MapAndAncestors(2), 0, LIMIT)
            .unwrap_err()
            .code,
        LIMIT
    );
    for selection in [MapSelection::All, MapSelection::MapAndAncestors(2)] {
        assert!(validate_domain_count(selection, 1, LIMIT).is_ok());
        assert!(validate_domain_count(selection, COVERAGE_MAX_MAPS, LIMIT).is_ok());
        assert_eq!(
            validate_domain_count(selection, COVERAGE_MAX_MAPS + 1, LIMIT)
                .unwrap_err()
                .code,
            LIMIT
        );
    }
}

#[test]
fn v22_route_packages_only_nonempty_models_named_by_placements_with_poison_godi_present() {
    let lab = crate::driver::v21_navigation_tests::Lab::new();
    let package = crate::driver::v21_navigation_tests::v22_geometry_package(&lab);
    let coverage = Coverage::parse(
        &std::fs::read(package.join(COVERAGE_PATH)).expect("v22 coverage artifact"),
    )
    .unwrap();

    assert_eq!(coverage.selection, MapSelection::MapAndAncestors(2));
    assert!(!package.join(AVAILABLE_COVERAGE_PATH).exists());
    assert_eq!(
        coverage.gameobject_scope,
        geometry::GameObjectScope::Excluded
    );
    assert!(!coverage.gameobject_request_made);
    assert_eq!(coverage.gameobject_startup_reads, 0);
    assert_eq!(coverage.model_scope, ModelScope::PlacementOnly);
    assert_eq!(
        coverage.liquid_object_scope,
        geometry::LiquidObjectScope::OutOfScope
    );
    assert!(!coverage.liquid_object_request_made);

    let model_artifacts: Vec<_> = coverage
        .artifacts
        .iter()
        .filter(|artifact| artifact.path.ends_with(".vmo"))
        .collect();
    assert_eq!(
        model_artifacts.len(),
        1,
        "a placement model must be present"
    );
    assert!(model_artifacts[0].bytes > 0);
    let placement_artifact = coverage
        .artifacts
        .iter()
        .find(|artifact| artifact.path == "vmaps/coverage/0002.placements")
        .expect("selected map placement proof");
    let placement_bytes = std::fs::read(package.join(&placement_artifact.path)).unwrap();
    let placements = forever_vmap::placement::DirBin::parse(
        &placement_bytes,
        forever_vmap::placement::Limits::default(),
    )
    .unwrap();
    assert_eq!(placements.record_count(), 1);
    let name = std::str::from_utf8(placements.records().next().unwrap().name()).unwrap();
    assert_eq!(model_artifacts[0].path, format!("vmaps/{name}.vmo"));
    let model = std::fs::read(package.join(&model_artifacts[0].path)).unwrap();
    forever_vmap::vmo::View::parse(&model, Default::default()).unwrap();
}

#[test]
fn vacated_wmo_coverage_v6_has_exact_version_and_round_trips() {
    let coverage = vacated_wmo_sample();
    let bytes = coverage.encode().unwrap();
    assert_eq!(
        u32::from_le_bytes(bytes[8..12].try_into().unwrap()),
        VACATED_WMO_COVERAGE_VERSION
    );
    let parsed = Coverage::parse_v33(&bytes).unwrap();
    assert_eq!(parsed.missing_m2, coverage.missing_m2);
    assert_eq!(parsed.missing_wmo, coverage.missing_wmo);
    assert_eq!(parsed.resource_profile, resources::Profile::RequestHeavyV1);
}

#[test]
fn available_coverage_v2_bytes_match_the_existing_golden() {
    let bytes = available_sample().encode().unwrap();
    assert_eq!(bytes.len(), 906);
    assert_eq!(hex(&bytes), AVAILABLE_V2_GOLDEN_HEX);
    assert!(Coverage::parse_v29(&bytes).is_ok());
}

#[test]
fn root_absent_coverage_v3_bytes_match_the_existing_golden() {
    let bytes = root_absent_sample().encode().unwrap();
    assert_eq!(bytes.len(), 915);
    assert_eq!(hex(&bytes), ROOT_ABSENT_V3_GOLDEN_HEX);
    assert!(Coverage::parse_v30(&bytes).is_ok());
}
