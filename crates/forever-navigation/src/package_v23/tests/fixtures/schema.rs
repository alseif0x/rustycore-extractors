// SPDX-License-Identifier: GPL-3.0-or-later
// Added 2026-10-20: synthetic V23 schema validation tests.
//! Canonical identity and coverage codec cases.

use super::*;

#[test]
fn selected_coverage_identity_and_codec_roundtrip_are_canonical() {
    let coverage = sample();
    coverage.validate().unwrap();
    assert_eq!(coverage.identity.plan_sha256, coverage.context.plan_sha256);
    assert_eq!(
        coverage.identity.terrain.context.components,
        Component::Terrain.bit()
    );
    assert_eq!(
        coverage.identity.vmap.context.components,
        Component::Collision.bit()
    );
    assert_eq!(coverage.identity.terrain_maps[0].known_present, 0);
    assert!(coverage.identity.terrain_maps[0].loaded);
    let identity = coverage.identity.render().unwrap();
    assert_eq!(
        SourceIdentity::parse(identity.as_bytes()).unwrap(),
        coverage.identity
    );
    assert_eq!(
        coverage.context.source_sha256,
        integrity::digest(identity.as_bytes()),
        "the context source hash binds the canonical stored identity"
    );

    let bytes = coverage.render().unwrap();
    let parsed = Coverage::parse(&bytes).unwrap();
    assert_eq!(parsed, coverage);
    assert_eq!(parsed.render().unwrap(), bytes);
    assert!(String::from_utf8(bytes).unwrap().contains(
        "tile\t0\t1\tempty\t0\t0000000000000000000000000000000000000000000000000000000000000000\n"
    ));
}

#[test]
fn coverage_and_identity_grammars_reject_unknown_duplicate_and_noncanonical_fields() {
    let coverage = sample();
    let rendered = coverage.render().unwrap();
    let text = String::from_utf8(rendered.clone()).unwrap();

    let mut unknown = text.clone();
    unknown.push_str("unknown-field\t1\n");
    parse_rejects_without_panicking(unknown.as_bytes());

    let duplicate = text.replace(
        "metadata-calls\t9\n",
        "metadata-calls\t9\nmetadata-calls\t9\n",
    );
    parse_rejects_without_panicking(duplicate.as_bytes());

    let noncanonical_tile = text.replace("tile\t0\t0\twritten\t12\t", "tile\t00\t0\twritten\t12\t");
    parse_rejects_without_panicking(noncanonical_tile.as_bytes());

    let identity = coverage.identity.render().unwrap();
    let identity_v1 = identity.replacen("identity\t2\n", "identity\t1\n", 1);
    parse_rejects_without_panicking(&rebind_identity_text(&coverage, identity_v1));
    let mut identity_unknown = identity.clone();
    identity_unknown.push_str("unknown-identity-field\t1\n");
    parse_rejects_without_panicking(&rebind_identity_text(&coverage, identity_unknown));

    let mut identity_duplicate = identity.clone();
    identity_duplicate.push_str("build\t70450\n");
    parse_rejects_without_panicking(&rebind_identity_text(&coverage, identity_duplicate));

    let identity_noncanonical = identity.replace("selected-map\t48\n", "selected-map\t048\n");
    parse_rejects_without_panicking(&rebind_identity_text(&coverage, identity_noncanonical));

    let wrong_identity_digest = replace_line(
        &rendered,
        "identity-sha256\t",
        &format!("identity-sha256\t{}\n", integrity::hex(&[0x99; 32])),
    );
    parse_rejects_without_panicking(&wrong_identity_digest);
}

#[test]
fn coverage_binds_build_locale_component_plan_producer_and_source_contexts() {
    let mut bad = sample();
    bad.context.plan_sha256 = [0xa1; 32];
    assert_rejected(bad);

    let mut bad = sample();
    bad.context.producer_sha256 = [0xa2; 32];
    assert_rejected(bad);

    let mut bad = sample();
    bad.context.components = Component::Terrain.bit();
    assert_rejected(bad);

    let mut bad = sample();
    bad.context.build = 70425;
    assert_rejected(bad);

    let mut bad = sample();
    bad.context.locale = Locale::EsEs;
    assert_rejected(bad);

    let mut bad = sample();
    bad.identity.current_source_sha256 = [0xb1; 32];
    assert_rejected(bad);

    let mut bad = sample();
    bad.identity.terrain.context.plan_sha256 = [0xb2; 32];
    assert_rejected(bad);

    let mut bad = sample();
    bad.identity.vmap.context.source_sha256 = [0xb3; 32];
    assert_rejected(bad);

    let mut bad = sample();
    bad.identity.locale = "esES".to_owned();
    refresh_identity_binding(&mut bad);
    assert!(
        bad.validate().is_err(),
        "identity locale must match context locale"
    );

    let mut identity = sample().identity;
    identity.plan_sha256 = [0xc1; 32];
    let bytes = rebind_identity_text(&sample(), identity.render().unwrap());
    parse_rejects_without_panicking(&bytes);
}

#[test]
fn map_liquid_catalog_authentication_and_record_counts_are_bounded() {
    let mut bad = sample();
    bad.identity.map.file_data_id = bad.identity.map.file_data_id.wrapping_add(1);
    assert_rejected(bad);

    let mut bad = sample();
    bad.identity.liquid.file_data_id = bad.identity.liquid.file_data_id.wrapping_add(1);
    assert_rejected(bad);

    let mut bad = sample();
    bad.identity.map.encoded_key_authenticated = false;
    assert_rejected(bad);

    let mut bad = sample();
    bad.identity.map.full_ckey_verified = true;
    assert_rejected(bad);

    let mut complete = sample();
    complete.identity.map.full_ckey_verified = true;
    complete.identity.map.available_bytes = complete.identity.map.declared_bytes;
    refresh_identity_binding(&mut complete);
    complete.validate().unwrap();
    complete.render().unwrap();

    let mut bad = sample();
    bad.identity.map.direct_rows = 2;
    assert_rejected(bad);

    let mut bad = sample();
    bad.identity.map.terrain_materialized_rows = 2;
    assert_rejected(bad);

    let mut bad = sample();
    bad.identity.map.vmap_materialized_rows = 0;
    assert_rejected(bad);

    let mut bad = sample();
    bad.identity.map.range_count = 0;
    assert_rejected(bad);

    let mut bad = sample();
    bad.identity.map.section_count = 17;
    assert_rejected(bad);

    let mut bad = sample();
    bad.identity.map.available_bytes = bad.identity.map.declared_bytes + 1;
    assert_rejected(bad);
}

#[test]
fn terrain_and_vmap_materialized_projection_counts_roundtrip_independently() {
    let mut coverage = sample();
    coverage.identity.map.copy_pairs = 1;
    coverage.identity.map.terrain_materialized_rows = 0;
    coverage.identity.map.vmap_materialized_rows = 2;
    refresh_identity_binding(&mut coverage);
    coverage.validate().unwrap();

    assert!(terrain_materialized_rows_match(&coverage.identity.map, 0));
    assert!(!terrain_materialized_rows_match(&coverage.identity.map, 2));
    assert!(vmap_materialized_rows_match(&coverage.identity.map, 2));
    assert!(!vmap_materialized_rows_match(&coverage.identity.map, 0));

    let decoded = Coverage::parse(&coverage.render().unwrap()).unwrap();
    assert_eq!(decoded.identity.map.terrain_materialized_rows, 0);
    assert_eq!(decoded.identity.map.vmap_materialized_rows, 2);
}

#[test]
fn closure_and_terrain_facts_require_sorted_known_consistent_loaded_rows() {
    let mut bad = sample();
    bad.identity.closure[1].output = false;
    assert_rejected(bad);

    let mut bad = sample();
    bad.identity.closure[0].output = true;
    assert_rejected(bad);

    let mut bad = sample();
    bad.identity.closure[0].is_parent = false;
    assert_rejected(bad);

    let mut bad = sample();
    bad.identity.closure[1].tiles = 1;
    assert_rejected(bad);

    let mut unsorted = sample();
    unsorted.identity.closure.swap(0, 1);
    assert_rejected(unsorted.clone());
    assert!(unsorted.render().is_err());

    let mut bad = sample();
    bad.identity.terrain_maps[0].loaded = false;
    assert_rejected(bad);

    let mut bad = sample();
    bad.identity.terrain_maps[0].known_present = 1;
    assert_rejected(bad);

    let mut bad = sample();
    bad.identity.terrain_maps[0].missing_root_adts = 1;
    assert_rejected(bad);

    let mut bad = sample();
    bad.identity.terrain_maps[0].id = 47;
    assert_rejected(bad);

    let mut bad = sample();
    bad.identity.closure[1].parent = Some(2);
    assert_rejected(bad);
}

#[test]
fn closure_requires_a_present_parentless_root_and_rejects_root_cycles() {
    let mut missing_root = sample();
    missing_root.identity.closure.remove(0);
    missing_root.identity.terrain_maps.remove(0);
    assert_rejected(missing_root);

    let mut root_self_parent = sample();
    root_self_parent.identity.closure[0].parent = Some(1);
    assert_rejected(root_self_parent);

    let mut root_child_cycle = sample();
    root_child_cycle.identity.closure[0].parent = Some(48);
    assert_rejected(root_child_cycle);
}

#[test]
fn tile_order_written_empty_hashes_and_global_output_totals_are_consistent() {
    let mut bad = sample();
    bad.tiles.swap(0, 1);
    assert_rejected(bad);

    let mut bad = sample();
    bad.tiles[1].tile = bad.tiles[0].tile;
    assert_rejected(bad);

    let mut bad = sample();
    bad.tiles[1].tile.row = 64;
    assert_rejected(bad);

    let mut bad = sample();
    bad.tiles[0].sha256 = [0; 32];
    assert_rejected(bad);

    let mut bad = sample();
    bad.tiles[1].sha256 = [0x91; 32];
    assert_rejected(bad);

    let mut bad = sample();
    bad.tiles[1].bytes = 1;
    assert_rejected(bad);

    let mut bad = sample();
    bad.global_sha256 = [0; 32];
    assert_rejected(bad);

    let mut bad = sample();
    bad.written_tiles = 2;
    assert_rejected(bad);

    let mut bad = sample();
    bad.maps = 2;
    assert_rejected(bad);

    let mut bad = sample();
    bad.output_bytes += 1;
    assert_rejected(bad);

    let mut overflow = sample();
    overflow.global_bytes = u64::MAX;
    let result = std::panic::catch_unwind(|| overflow.validate());
    assert!(result.is_ok(), "global-plus-tile byte overflow panicked");
    assert!(result.unwrap().is_err());
}

#[test]
fn hostile_maximum_counters_are_rejected_without_panics() {
    let base = sample();
    let bytes = base.render().unwrap();
    let text = String::from_utf8(bytes.clone()).unwrap();

    let huge_total = text.replace("global\t48\t40\t", &format!("global\t48\t{}\t", u64::MAX));
    parse_rejects_without_panicking(huge_total.as_bytes());

    let huge_tile = text.replace(
        "tile\t0\t0\twritten\t12\t",
        &format!("tile\t0\t0\twritten\t{}\t", u64::MAX),
    );
    parse_rejects_without_panicking(huge_tile.as_bytes());

    let huge_finished_map_count =
        text.replace("finish\t1\t2\t", &format!("finish\t{}\t2\t", usize::MAX));
    parse_rejects_without_panicking(huge_finished_map_count.as_bytes());

    let overflowing_tile_totals = text.replace(
        "finish\t1\t2\t1\t1\t",
        &format!("finish\t1\t2\t{}\t1\t", usize::MAX),
    );
    parse_rejects_without_panicking(overflowing_tile_totals.as_bytes());

    let mut identity = base.identity.clone();
    identity.closure[0].tiles = usize::MAX;
    parse_rejects_without_panicking(&rebind_identity_text(&base, identity.render().unwrap()));

    let mut identity = base.identity.clone();
    identity.map.available_bytes = u64::MAX;
    parse_rejects_without_panicking(&rebind_identity_text(&base, identity.render().unwrap()));
}
