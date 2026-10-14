// SPDX-License-Identifier: GPL-3.0-or-later
//! Authored fixtures only; no client bytes or runtime acceptance.
use super::*;
use forever_casc::{Locale, integrity, local_index, package, root, terrain};
use std::{
    fs,
    path::PathBuf,
    sync::atomic::{AtomicU64, Ordering},
};

fn plan(selection: Selection) -> Plan {
    Plan {
        publication_profile: crate::plan::PublicationProfile::Legacy,
        build: 70205,
        schema_catalog: crate::plan::SchemaCatalog::PinnedMetadataV1,
        decryption: crate::plan::Decryption::Disabled,
        collision_storage: crate::plan::CollisionStorage::Resident,
        locale: Locale::EnUs,
        index_layout: local_index::Layout::Contiguous,
        root_layout: root::Layout::Header50893V2,
        components: Component::Terrain.bit(),
        db2: vec![],
        terrain: selection,
        navigation: Selection::None,
        pack: true,
        big_base_unit: false,
        skip_liquids: false,
        maximum_slope: 55,
        non_steep_slope: 55,
        offmesh: None,
        map_domain: None,
    }
}
fn artifact(component: Component, path: &str, bytes: &[u8]) -> Artifact {
    Artifact {
        component,
        path: path.into(),
        bytes: bytes.len() as u64,
        sha256: integrity::digest(bytes),
    }
}
fn list(tiles: &[(u8, u8)]) -> Vec<u8> {
    let mut list = terrain::tilelist::TileList::new(70205).unwrap();
    for &(x, y) in tiles {
        list.set(x, y).unwrap();
    }
    list.bytes().to_vec()
}
fn observe(facts: &mut Facts, component: Component, path: &str, bytes: &[u8]) {
    facts
        .inspect(&artifact(component, path, bytes), bytes)
        .unwrap();
}
fn maps(facts: &mut Facts, map: u32, x: u8, y: u8) {
    observe(
        facts,
        Component::Terrain,
        &format!("maps/{map:04}_{y:02}_{x:02}.map"),
        &[],
    );
}
fn tilelist(facts: &mut Facts, map: u32, tiles: &[(u8, u8)]) {
    observe(
        facts,
        Component::Terrain,
        &format!("maps/{map:04}.tilelist"),
        &list(tiles),
    );
}
fn navigation(facts: &mut Facts, map: u32, global: bool) {
    let path = if global {
        format!("mmaps/{map:04}.mmap")
    } else {
        format!("mmaps/{map:04}_03_07.mmtile")
    };
    observe(facts, Component::Navigation, &path, &[]);
}
fn result(facts: Facts, selection: Selection) -> Result<(), Error> {
    facts.finish(&plan(selection))
}

#[test]
fn asymmetric_reversed_ascii_bits_match_maps_and_edges() {
    let mut facts = Facts::default();
    let tiles = [(7, 3), (0, 0), (63, 63), (0, 63), (63, 0)];
    tilelist(&mut facts, 42, &tiles);
    for (x, y) in tiles {
        maps(&mut facts, 42, x, y);
    }
    let raw = list(&[(7, 3)]);
    assert_eq!(raw[12 + 4095 - (3 * 64 + 7)], b'1');
    assert_eq!(raw[12 + (3 * 64 + 7)], b'0');
    result(facts, Selection::All).unwrap();
}
#[test]
fn transposed_coordinates_fail_tile_set_equality() {
    let mut facts = Facts::default();
    tilelist(&mut facts, 42, &[(3, 7)]);
    maps(&mut facts, 42, 7, 3);
    assert_eq!(
        result(facts, Selection::All).unwrap_err(),
        error("product_links_terrain_tile_set", Component::Terrain)
    );
}
#[test]
fn maps_without_list_rejects_for_all_and_map() {
    for selection in [Selection::All, Selection::Map(42)] {
        let mut facts = Facts::default();
        maps(&mut facts, 42, 7, 3);
        assert_eq!(
            result(facts, selection).unwrap_err(),
            error("product_links_terrain_missing_list", Component::Terrain)
        );
    }
}
#[test]
fn populated_list_without_maps_rejects() {
    let mut facts = Facts::default();
    tilelist(&mut facts, 42, &[(7, 3)]);
    assert_eq!(
        result(facts, Selection::Map(42)).unwrap_err().code,
        "product_links_terrain_tile_set"
    );
}
#[test]
fn empty_list_without_maps_is_valid() {
    let mut facts = Facts::default();
    tilelist(&mut facts, 42, &[]);
    result(facts, Selection::All).unwrap();
}
#[test]
fn extra_published_maps_and_missing_listed_maps_each_reject() {
    for (present, listed) in [
        (&[(7, 3), (8, 3)][..], &[(7, 3)][..]),
        (&[(7, 3)][..], &[(7, 3), (8, 3)][..]),
    ] {
        let mut facts = Facts::default();
        tilelist(&mut facts, 42, listed);
        for &(x, y) in present {
            maps(&mut facts, 42, x, y);
        }
        assert_eq!(
            result(facts, Selection::All).unwrap_err().code,
            "product_links_terrain_tile_set"
        );
    }
}
#[test]
fn map_selection_checks_observed_map_facts() {
    let mut facts = Facts::default();
    maps(&mut facts, 42, 7, 3);
    tilelist(&mut facts, 42, &[(7, 3)]);
    result(facts, Selection::Map(42)).unwrap();
}
#[test]
fn tile_selection_needs_no_whole_map_list() {
    let mut facts = Facts::default();
    maps(&mut facts, 42, 7, 3);
    result(
        facts,
        Selection::Tile {
            map: 42,
            x: 7,
            y: 3,
        },
    )
    .unwrap();
}
#[test]
fn entirely_unobserved_maps_have_no_completeness_claim() {
    result(Facts::default(), Selection::All).unwrap();
    result(Facts::default(), Selection::Map(42)).unwrap();
}
#[test]
fn navigation_tile_requires_its_own_global() {
    let mut facts = Facts::default();
    navigation(&mut facts, 42, false);
    navigation(&mut facts, 43, true);
    assert_eq!(
        result(facts, Selection::None).unwrap_err(),
        error(
            "product_links_navigation_missing_global",
            Component::Navigation
        )
    );
}
#[test]
fn navigation_only_facts_accept_matching_global_without_terrain_binding() {
    let mut facts = Facts::default();
    navigation(&mut facts, 42, false);
    navigation(&mut facts, 42, true);
    result(facts, Selection::All).unwrap();
}
#[test]
fn global_only_needs_no_tile() {
    let mut facts = Facts::default();
    navigation(&mut facts, 42, true);
    result(facts, Selection::All).unwrap();
}
#[test]
fn facts_are_independent_of_artifact_order() {
    for reverse in [false, true] {
        let mut facts = Facts::default();
        if reverse {
            navigation(&mut facts, 42, false);
            maps(&mut facts, 42, 7, 3);
        }
        tilelist(&mut facts, 42, &[(7, 3)]);
        navigation(&mut facts, 42, true);
        if !reverse {
            maps(&mut facts, 42, 7, 3);
            navigation(&mut facts, 42, false);
        }
        result(facts, Selection::All).unwrap();
    }
}
#[test]
fn unrelated_components_add_no_map_facts() {
    let mut facts = Facts::default();
    for component in [
        Component::Db2,
        Component::GameTables,
        Component::Cameras,
        Component::Collision,
    ] {
        observe(&mut facts, component, "unrelated", &[]);
    }
    assert!(facts.maps.is_empty());
    result(facts, Selection::All).unwrap();
}
#[test]
fn map_limit_rejects_before_insertion_and_existing_ids_remain_admitted() {
    let mut facts = Facts::default();
    for id in 0..MAX_LINK_MAPS as u32 {
        facts.map(id).unwrap();
    }
    assert!(facts.maps.capacity() * std::mem::size_of::<MapFacts>() <= MAX_LOGICAL_BYTES);
    let capacity = facts.maps.capacity();
    assert_eq!(
        facts.map(MAX_LINK_MAPS as u32).err().unwrap().code,
        "product_links_map_limit"
    );
    assert_eq!(facts.maps.len(), MAX_LINK_MAPS);
    assert_eq!(facts.maps.capacity(), capacity);
    facts.map(0).unwrap();
    facts.map(MAX_LINK_MAPS as u32 - 1).unwrap();
}
#[test]
fn sorted_insertions_preserve_distinct_map_facts() {
    let mut facts = Facts::default();
    for id in [90, 1, 42, 0, 43] {
        navigation(&mut facts, id, true);
    }
    assert_eq!(
        facts.maps.iter().map(|map| map.id).collect::<Vec<_>>(),
        [0, 1, 42, 43, 90]
    );
    result(facts, Selection::All).unwrap();
}

static NEXT: AtomicU64 = AtomicU64::new(0);
struct Lab(PathBuf);
impl Lab {
    fn new() -> Self {
        let path = std::env::temp_dir().join(format!(
            "rce-links-authored-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        assert!(!path.ancestors().any(|p| p.join(".git").exists()));
        fs::create_dir(&path).unwrap();
        Self(path)
    }
    fn publish(&self, plan: &Plan, files: &[(&str, Vec<u8>)]) -> package::Summary {
        let context = package::Context {
            build: plan.build,
            locale: plan.locale,
            components: plan.components,
            source_sha256: [1; 32],
            plan_sha256: integrity::digest(plan.render().as_bytes()),
            producer_sha256: [3; 32],
        };
        let mut run =
            package::Run::create(&self.0.join("output"), context, &[], Default::default()).unwrap();
        for (path, bytes) in files {
            run.put(Component::Terrain, path, bytes).unwrap();
        }
        run.finish_component(Component::Terrain).unwrap();
        run.finish().unwrap()
    }
}
impl Drop for Lab {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.0).unwrap();
    }
}
fn map_bytes() -> Vec<u8> {
    use forever_terrain::{height, maps};
    let area = maps::AreaSection::from_ids(&[0; 256]);
    let height = height::HeightSection::from_bits(
        &[0; height::V9_COUNT],
        &[0; height::V8_COUNT],
        Default::default(),
        None,
        height::MAX_SECTION_BYTES,
    )
    .unwrap();
    maps::assemble(
        70205,
        maps::MapSections {
            area: &area,
            height: &height,
            liquid: None,
            holes: None,
        },
        maps::MAX_MAP_BYTES,
    )
    .unwrap()
}
#[test]
fn complete_package_links_pass_and_keep_the_existing_summary() {
    let lab = Lab::new();
    let plan = plan(Selection::All);
    let expected = lab.publish(
        &plan,
        &[
            ("maps/0042.tilelist", list(&[(7, 3)])),
            ("maps/0042_03_07.map", map_bytes()),
        ],
    );
    assert_eq!(
        super::super::verify(&plan, &lab.0.join("output"), None).unwrap(),
        expected
    );
    assert_eq!(
        super::super::verify_links(&plan, &lab.0.join("output"), None).unwrap(),
        expected
    );
}
#[test]
fn intrinsic_verification_still_accepts_unlinked_packages() {
    for files in [
        vec![("maps/0042.tilelist", list(&[(7, 3)]))],
        vec![("maps/0042_03_07.map", map_bytes())],
    ] {
        let lab = Lab::new();
        let plan = plan(Selection::All);
        lab.publish(&plan, &files);
        super::super::verify(&plan, &lab.0.join("output"), None).unwrap();
        assert!(super::super::verify_links(&plan, &lab.0.join("output"), None).is_err());
    }
}
#[test]
fn empty_tilelist_package_passes_links() {
    let lab = Lab::new();
    let plan = plan(Selection::All);
    lab.publish(&plan, &[("maps/0042.tilelist", list(&[]))]);
    super::super::verify_links(&plan, &lab.0.join("output"), None).unwrap();
}
#[test]
fn tile_selection_still_rejects_whole_map_list_intrinsically() {
    let lab = Lab::new();
    let plan = plan(Selection::Tile {
        map: 42,
        x: 7,
        y: 3,
    });
    lab.publish(&plan, &[("maps/0042.tilelist", list(&[]))]);
    let error = super::super::verify_links(&plan, &lab.0.join("output"), None).unwrap_err();
    assert_eq!(error.code, "product_selection_mismatch");
    assert_eq!(error.component, Some(Component::Terrain));
}
#[test]
fn malformed_intrinsic_bytes_precede_link_checks() {
    let lab = Lab::new();
    let plan = plan(Selection::All);
    lab.publish(&plan, &[("maps/0042_03_07.map", b"invalid MAPS".to_vec())]);
    let original = super::super::verify(&plan, &lab.0.join("output"), None).unwrap_err();
    assert_ne!(original.code, "product_links_terrain_missing_list");
    assert_eq!(
        super::super::verify_links(&plan, &lab.0.join("output"), None).unwrap_err(),
        original
    );
}

#[test]
fn expected_context_plan_binding_and_digest_failures_match_intrinsic_verifier() {
    let lab = Lab::new();
    let plan = plan(Selection::All);
    let summary = lab.publish(&plan, &[("maps/0042.tilelist", list(&[]))]);
    let output = lab.0.join("output");
    let expected = package::Context {
        source_sha256: [9; 32],
        ..summary.context
    };
    assert_eq!(
        super::super::verify_links(&plan, &output, Some(expected)).unwrap_err(),
        super::super::verify(&plan, &output, Some(expected)).unwrap_err()
    );
    let changed = Plan {
        pack: false,
        ..plan.clone()
    };
    assert_eq!(
        super::super::verify_links(&changed, &output, None)
            .unwrap_err()
            .code,
        "product_plan_mismatch"
    );
    fs::write(output.join("maps/0042.tilelist"), list(&[(7, 3)])).unwrap();
    assert_eq!(
        super::super::verify_links(&plan, &output, None).unwrap_err(),
        super::super::verify(&plan, &output, None).unwrap_err()
    );
}
#[test]
fn invalid_plan_rejects_before_output_io_in_links_mode() {
    let mut plan = plan(Selection::All);
    plan.components = 0;
    let path = std::path::Path::new("/nonexistent/forever-links-invalid-plan");
    assert_eq!(
        super::super::verify_links(&plan, path, None).unwrap_err(),
        super::super::verify(&plan, path, None).unwrap_err()
    );
}
