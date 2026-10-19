// SPDX-License-Identifier: GPL-3.0-or-later
// Modified by the RustyCore Extractors project, 2026-10-19.
//! Synthetic v22 producer-coverage codec checks; no client files are used.
use super::*;
use forever_casc::Locale;

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
        requested_map: 2,
        gameobject_scope: geometry::GameObjectScope::Excluded,
        gameobject_file_data_id: forever_casc::collision::gameobjects::TABLE_FILE_DATA_ID,
        gameobject_block_reason: geometry::GameObjectBlockReason::ExcludedByProfile,
        gameobject_request_made: false,
        gameobject_startup_reads: 0,
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
        }],
        artifacts: vec![ArtifactFact {
            path: "vmaps/0002/0002.vmtree".into(),
            bytes: 16,
            sha: [0x34; 32],
        }],
    }
}

#[test]
fn coverage_round_trip_retains_partial_map_scope_and_declared_policy_facts() {
    let coverage = sample();
    let bytes = coverage.encode().unwrap();
    let parsed = Coverage::parse(&bytes).unwrap();
    assert_eq!(parsed.context, coverage.context);
    assert_eq!(parsed.requested_map, 2);
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
fn coverage_codec_rejects_bad_policy_bits_truncation_and_noncanonical_paths() {
    let bytes = sample().encode().unwrap();
    assert!(Coverage::parse(&bytes[..bytes.len() - 1]).is_err());

    let mut bad_facts = bytes.clone();
    // Magic(8), version(4), context length/context, selection(4), then the
    // The plan/source/domain/policy strings and declared source facts precede
    // the GameObjectDisplayInfo file ID and fixed fact-bit word.
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
fn v22_route_packages_only_nonempty_models_named_by_placements_with_poison_godi_present() {
    let lab = crate::driver::v21_navigation_tests::Lab::new();
    let package = crate::driver::v21_navigation_tests::v22_geometry_package(&lab);
    let coverage = Coverage::parse(
        &std::fs::read(package.join(COVERAGE_PATH)).expect("v22 coverage artifact"),
    )
    .unwrap();

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
