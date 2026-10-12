// SPDX-License-Identifier: GPL-3.0-or-later
// Authored 2026-10-11: synthetic product inventory/format checks only.
//! No client acquisition, native parity, complete geometry or runtime claim.
use super::*;
use crate::plan::{Entry, Selection};
use forever_casc::{Locale, local_index, package::Run, root};
use forever_db2::{CharacterTable, DecodedTable, catalog};
use std::{
    fs,
    path::PathBuf,
    sync::atomic::{AtomicU64, Ordering},
};

static NEXT: AtomicU64 = AtomicU64::new(0);
struct Lab(PathBuf);
impl Lab {
    fn new() -> Self {
        let mut base = std::env::temp_dir();
        if base.ancestors().any(|path| path.join(".git").exists()) {
            base = PathBuf::from("/tmp");
        }
        let path = base.join(format!(
            "rce-product-verify-synthetic-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&path).unwrap();
        Self(path)
    }
    fn output(&self) -> PathBuf {
        self.0.join("package")
    }
    fn publish(
        &self,
        context: Context,
        files: &[(Component, String, Vec<u8>)],
    ) -> package::Summary {
        let mut run = Run::create(&self.output(), context, &[], Default::default()).unwrap();
        for (component, path, bytes) in files {
            run.put(*component, path, bytes).unwrap();
        }
        for component in Component::ALL {
            if context.contains(component) {
                run.finish_component(component).unwrap();
            }
        }
        let summary = run.finish().unwrap();
        // Every negative below starts with a valid complete common inventory.
        assert_eq!(
            package::verify(&self.output(), Some(context), Default::default()).unwrap(),
            summary
        );
        summary
    }
}
impl Drop for Lab {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.0).unwrap();
    }
}
fn entry(name: &str) -> Entry {
    Entry {
        name: name.into(),
        id: catalog::find(name).unwrap().file_data_id,
    }
}
fn db2_plan() -> Plan {
    Plan {
        build: 70205,
        decryption: crate::plan::Decryption::Disabled,
        collision_storage: crate::plan::CollisionStorage::Resident,
        locale: Locale::EnUs,
        index_layout: local_index::Layout::Contiguous,
        root_layout: root::Layout::Header50893V2,
        components: Component::Db2.bit(),
        db2: vec![entry("SpellCastTimes")],
        terrain: Selection::None,
        navigation: Selection::None,
        pack: true,
        big_base_unit: false,
        skip_liquids: false,
        maximum_slope: 55,
        non_steep_slope: 55,
    }
}
fn gt_plan() -> Plan {
    Plan {
        components: Component::GameTables.bit(),
        db2: vec![],
        ..db2_plan()
    }
}
fn camera_plan() -> Plan {
    Plan {
        components: Component::Db2.bit() | Component::Cameras.bit(),
        db2: vec![entry("CinematicCamera")],
        ..db2_plan()
    }
}
fn context(plan: &Plan) -> Context {
    Context {
        build: plan.build,
        locale: plan.locale,
        components: plan.components,
        source_sha256: [1; 32],
        plan_sha256: integrity::digest(plan.render().as_bytes()),
        producer_sha256: [3; 32],
    }
}
fn put32(bytes: &mut [u8], offset: usize, value: u32) {
    bytes[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
}
/// Complete zero-section WDC5 with every primitive metadata slot present.
/// Schema identity comes from the public pinned schema, not a client file.
fn empty_db2(name: &str) -> Vec<u8> {
    let metadata = catalog::find(name).unwrap();
    let table = CharacterTable::from_catalog(name).unwrap().unwrap();
    let (hash, layout, _, _) = table.schema_identity();
    let mut bytes = vec![0; 204 + 4 * metadata.file_field_count];
    bytes[..4].copy_from_slice(b"WDC5");
    put32(&mut bytes, 4, 5);
    put32(&mut bytes, 140, metadata.file_field_count as u32);
    put32(&mut bytes, 152, hash);
    put32(&mut bytes, 156, layout);
    put32(&mut bytes, 176, metadata.file_field_count as u32);
    assert!(DecodedTable::read(&bytes, table).unwrap().rows().is_empty());
    bytes
}
fn db2_file(name: &str, bytes: Vec<u8>) -> (Component, String, Vec<u8>) {
    (Component::Db2, format!("dbc/enUS/{name}.db2"), bytes)
}
fn gt_bytes(table: gt::Table) -> Vec<u8> {
    if let Some(columns) = table.value_columns() {
        let header = (0..columns)
            .map(|i| format!("v{i}"))
            .collect::<Vec<_>>()
            .join("\t");
        let values = vec!["-0"; columns].join("\t");
        format!("ignored\t{header}\nignored\t{values}\n").into_bytes()
    } else {
        b"opaque authored raw bytes\0\xff\tNaN\n".to_vec()
    }
}
fn gt_files() -> Vec<(Component, String, Vec<u8>)> {
    gt::Table::ALL
        .into_iter()
        .map(|table| {
            (
                Component::GameTables,
                table.relative_path().into(),
                gt_bytes(table),
            )
        })
        .collect()
}
fn artifact(component: Component, path: &str, bytes: &[u8]) -> Artifact {
    Artifact {
        component,
        path: path.into(),
        bytes: bytes.len() as u64,
        sha256: integrity::digest(bytes),
    }
}
/// One regular direct camera record, with explicit primitive/storage metadata.
fn camera_source(id: u32) -> Vec<u8> {
    let mut bytes = vec![0; 384 + 28 + 4];
    bytes[..4].copy_from_slice(b"WDC5");
    for (offset, value) in [
        (4, 5),
        (136, 1),
        (140, 5),
        (144, 28),
        (156, cameras::SOURCE_LAYOUT),
        (160, 1),
        (164, 1),
        (176, 5),
        (188, 120),
        (200, 1),
        (212, 384),
        (216, 1),
        (228, 4),
        (384 + 20, id),
        (384 + 28, 1),
    ] {
        put32(&mut bytes, offset, value);
    }
    bytes[174..176].copy_from_slice(&u16::MAX.to_le_bytes());
    for (field, (offset, count)) in [(0u16, 3), (96, 1), (128, 1), (160, 1), (192, 1)]
        .into_iter()
        .enumerate()
    {
        let at = 264 + 24 * field;
        bytes[at..at + 2].copy_from_slice(&offset.to_le_bytes());
        put32(&mut bytes, at + 16, 32);
        put32(&mut bytes, at + 20, count);
    }
    assert_eq!(
        cameras::discover(&bytes).unwrap().file_ids(),
        &BTreeSet::from([id])
    );
    bytes
}

#[test]
fn complete_empty_numeric_db2_subset_is_verified() {
    let lab = Lab::new();
    let plan = db2_plan();
    plan.validate().unwrap();
    let summary = lab.publish(
        context(&plan),
        &[db2_file("SpellCastTimes", empty_db2("SpellCastTimes"))],
    );
    assert_eq!(
        verify(&plan, &lab.output(), Some(context(&plan))).unwrap(),
        summary
    );
    assert_eq!(verify(&plan, &lab.output(), None).unwrap(), summary);
    assert_eq!(summary.files, 1);
    assert!(summary.json().contains("\"scope\":\"artifact_integrity\""));
}
#[test]
fn complete_gt_catalog_preserves_numeric_and_raw_only_admission() {
    let lab = Lab::new();
    let plan = gt_plan();
    let files = gt_files();
    assert_eq!(files.len(), 20);
    assert_eq!(
        gt::Table::ALL
            .into_iter()
            .filter(|table| table.value_columns().is_none())
            .count(),
        6
    );
    let summary = lab.publish(context(&plan), &files);
    assert_eq!(
        verify(&plan, &lab.output(), Some(context(&plan))).unwrap(),
        summary
    );
    assert_eq!(summary.files, 20);
}
#[test]
fn empty_camera_discovery_requires_only_complete_source_db2() {
    let lab = Lab::new();
    let plan = camera_plan();
    let summary = lab.publish(
        context(&plan),
        &[db2_file("CinematicCamera", empty_db2("CinematicCamera"))],
    );
    assert_eq!(verify(&plan, &lab.output(), None).unwrap(), summary);
}
#[test]
fn camera_raw_bytes_match_discovered_inventory_without_m2_parsing() {
    let lab = Lab::new();
    let plan = camera_plan();
    let summary = lab.publish(
        context(&plan),
        &[
            db2_file("CinematicCamera", camera_source(11)),
            (
                Component::Cameras,
                cameras::relative_path(11),
                b"authored opaque, not M2".to_vec(),
            ),
        ],
    );
    assert_eq!(verify(&plan, &lab.output(), None).unwrap(), summary);
    assert_eq!(summary.files, 2);
}
#[test]
fn expected_context_rejects_identity_mismatches() {
    let lab = Lab::new();
    let plan = db2_plan();
    let ctx = context(&plan);
    lab.publish(
        ctx,
        &[db2_file("SpellCastTimes", empty_db2("SpellCastTimes"))],
    );
    for expected in [
        Context {
            source_sha256: [4; 32],
            ..ctx
        },
        Context {
            producer_sha256: [4; 32],
            ..ctx
        },
        Context {
            plan_sha256: [4; 32],
            ..ctx
        },
        Context {
            build: 70170,
            ..ctx
        },
        Context {
            locale: Locale::EsEs,
            ..ctx
        },
        Context {
            components: Component::GameTables.bit(),
            ..ctx
        },
    ] {
        assert_eq!(
            verify(&plan, &lab.output(), Some(expected))
                .unwrap_err()
                .code,
            "package_context"
        );
    }
}
#[test]
fn supplied_plan_must_match_every_common_context_selection_fact() {
    let plan = gt_plan();
    let ctx = context(&plan);
    for changed in [
        Context {
            build: 70170,
            ..ctx
        },
        Context {
            locale: Locale::EsEs,
            ..ctx
        },
        Context {
            components: Component::Db2.bit(),
            ..ctx
        },
        Context {
            plan_sha256: [4; 32],
            ..ctx
        },
    ] {
        let lab = Lab::new();
        lab.publish(changed, &[]);
        assert_eq!(
            verify(&plan, &lab.output(), None).unwrap_err().code,
            "product_plan_mismatch"
        );
    }
}
#[test]
fn changing_even_non_geometry_plan_option_rejects_original_plan_binding() {
    let lab = Lab::new();
    let plan = db2_plan();
    lab.publish(
        context(&plan),
        &[db2_file("SpellCastTimes", empty_db2("SpellCastTimes"))],
    );
    let changed = Plan {
        pack: false,
        ..plan
    };
    assert_eq!(
        verify(&changed, &lab.output(), None).unwrap_err().code,
        "product_plan_mismatch"
    );
}
#[test]
fn invalid_plan_rejects_before_output_io() {
    let plan = Plan {
        components: 0,
        ..db2_plan()
    };
    assert_eq!(
        verify(
            &plan,
            Path::new("/nonexistent/product-plan-validation"),
            None
        )
        .unwrap_err()
        .code,
        "product_plan_components"
    );
}
#[test]
fn missing_selected_db2_rejects_valid_empty_common_package() {
    let lab = Lab::new();
    let plan = db2_plan();
    lab.publish(context(&plan), &[]);
    assert_eq!(
        verify(&plan, &lab.output(), None).unwrap_err().code,
        "product_inventory"
    );
}
#[test]
fn missing_gt_file_rejects_otherwise_complete_catalog() {
    let lab = Lab::new();
    let plan = gt_plan();
    let mut files = gt_files();
    files.pop();
    lab.publish(context(&plan), &files);
    assert_eq!(
        verify(&plan, &lab.output(), None).unwrap_err().code,
        "product_inventory"
    );
}
#[test]
fn missing_camera_file_rejects_complete_source_and_common_inventory() {
    let lab = Lab::new();
    let plan = camera_plan();
    lab.publish(
        context(&plan),
        &[db2_file("CinematicCamera", camera_source(11))],
    );
    assert_eq!(
        verify(&plan, &lab.output(), None).unwrap_err().code,
        "product_inventory"
    );
}
#[test]
fn extra_camera_file_rejects_discovery_mismatch() {
    let lab = Lab::new();
    let plan = camera_plan();
    lab.publish(
        context(&plan),
        &[
            db2_file("CinematicCamera", empty_db2("CinematicCamera")),
            (
                Component::Cameras,
                cameras::relative_path(11),
                b"authored raw".to_vec(),
            ),
        ],
    );
    assert_eq!(
        verify(&plan, &lab.output(), None).unwrap_err().code,
        "product_inventory"
    );
}
#[test]
fn missing_camera_source_db2_rejects_sealed_empty_components() {
    let lab = Lab::new();
    let plan = camera_plan();
    lab.publish(context(&plan), &[]);
    assert!(verify(&plan, &lab.output(), None).is_err());
}
#[test]
fn unknown_db2_and_gt_artifacts_reject_valid_common_inventory() {
    for (plan, file, expected) in [
        (
            db2_plan(),
            db2_file("NotInPlan", empty_db2("SpellCastTimes")),
            "product_inventory",
        ),
        (
            gt_plan(),
            (
                Component::GameTables,
                "gt/unknown.txt".into(),
                b"authored".to_vec(),
            ),
            "product_artifact_name",
        ),
    ] {
        let lab = Lab::new();
        lab.publish(context(&plan), &[file]);
        assert_eq!(
            verify(&plan, &lab.output(), None).unwrap_err().code,
            expected
        );
    }
}
#[test]
fn malformed_db2_rejects_despite_valid_common_digest_and_seals() {
    let lab = Lab::new();
    let plan = db2_plan();
    lab.publish(
        context(&plan),
        &[db2_file(
            "SpellCastTimes",
            b"authored, invalid WDC5".to_vec(),
        )],
    );
    let error = verify(&plan, &lab.output(), None).unwrap_err();
    assert_eq!(error.component, Some(Component::Db2));
    assert_ne!(error.code, "product_inventory");
}
#[test]
fn malformed_numeric_gt_rejects_without_treating_nan_as_raw() {
    let lab = Lab::new();
    let plan = gt_plan();
    let table = gt::Table::BarberShopCostBase;
    lab.publish(
        context(&plan),
        &[(
            Component::GameTables,
            table.relative_path().into(),
            b"id\tv\n1\tNaN\n".to_vec(),
        )],
    );
    let error = verify(&plan, &lab.output(), None).unwrap_err();
    assert_eq!(error.component, Some(Component::GameTables));
    assert_eq!(
        error.code,
        gt::parse_numeric(b"id\tv\n1\tNaN\n", table)
            .err()
            .unwrap()
            .code()
    );
}
#[test]
fn empty_payload_rejects_even_for_opaque_camera_files() {
    let lab = Lab::new();
    let plan = camera_plan();
    lab.publish(
        context(&plan),
        &[
            db2_file("CinematicCamera", camera_source(11)),
            (Component::Cameras, cameras::relative_path(11), vec![]),
        ],
    );
    let error = verify(&plan, &lab.output(), None).unwrap_err();
    assert_eq!(
        (error.code, error.component),
        ("product_empty_artifact", Some(Component::Cameras))
    );
}
#[test]
fn db2_and_camera_artifact_names_have_exact_canonical_case_and_spelling() {
    let db2 = empty_db2("SpellCastTimes");
    for path in [
        "dbc/enUS/SpellCastTimes.DB2",
        "dbc/enUS/spellcasttimes.db2",
        "dbc/enUS/sub/SpellCastTimes.db2",
    ] {
        let lab = Lab::new();
        let plan = db2_plan();
        lab.publish(
            context(&plan),
            &[(Component::Db2, path.into(), db2.clone())],
        );
        assert!(verify(&plan, &lab.output(), None).is_err());
    }
    for path in [
        "cameras/file0000000B.xxx",
        "cameras/FILE0000000b.xxx",
        "cameras/FILEB.xxx",
        "cameras/FILE0000000B.XXX",
        "cameras/sub/FILE0000000B.xxx",
    ] {
        let lab = Lab::new();
        let plan = camera_plan();
        lab.publish(
            context(&plan),
            &[
                db2_file("CinematicCamera", camera_source(11)),
                (Component::Cameras, path.into(), b"authored raw".to_vec()),
            ],
        );
        assert_eq!(
            verify(&plan, &lab.output(), None).unwrap_err().code,
            "product_artifact_name"
        );
    }
}
#[test]
fn terrain_and_collision_filename_checks_preserve_coordinate_order_and_bounds() {
    assert!(terrain_map_name("maps/0042_03_07.map").is_ok());
    assert!(collision_tile_name("0042_32767_255.vmtileidx", 42).is_ok());
    for path in [
        "maps/42_03_07.map",
        "maps/0042_3_07.map",
        "maps/0042_03_64.map",
        "maps/0042_64_03.map",
        "maps/0042_03_07.MAP",
        "maps/0042_03_07_00.map",
    ] {
        assert_eq!(
            terrain_map_name(path).unwrap_err().code,
            "product_artifact_name"
        );
    }
    for path in [
        "42_03_07.vmtile",
        "0042_3_07.vmtile",
        "0043_03_07.vmtile",
        "0042_32768_00.vmtile",
        "0042_00_256.vmtile",
        "0042_03_07.VMTILE",
        "0042_03_07.vmtileidx.extra",
    ] {
        assert_eq!(
            collision_tile_name(path, 42).unwrap_err().code,
            "product_artifact_name"
        );
    }
}
#[test]
fn malformed_intrinsic_formats_and_unknown_component_names_reject() {
    let plan = Plan {
        components: 63,
        db2: vec![entry("CinematicCamera"), entry("LiquidType"), entry("Map")],
        terrain: Selection::All,
        navigation: Selection::All,
        ..db2_plan()
    };
    plan.validate().unwrap();
    let bytes = b"authored invalid format";
    for (component, path) in [
        (Component::Terrain, "maps/0001_00_00.map"),
        (Component::Terrain, "maps/0001.tilelist"),
        (Component::Collision, "vmaps/GameObjectModels.dtree"),
        (Component::Collision, "vmaps/authored.vmo"),
        (Component::Collision, "vmaps/0001/0001.vmtree"),
        (Component::Navigation, "mmaps/0001.mmap"),
        (Component::Navigation, "mmaps/0001_00_00.mmtile"),
    ] {
        assert!(
            Facts::default()
                .inspect(&plan, &artifact(component, path, bytes), bytes)
                .is_err(),
            "{path}"
        );
    }
    for (component, path) in [
        (Component::Terrain, "maps/unknown.bin"),
        (Component::Collision, "vmaps/unknown.bin"),
        (Component::Navigation, "mmaps/unknown.bin"),
    ] {
        assert_eq!(
            Facts::default()
                .inspect(&plan, &artifact(component, path, bytes), bytes)
                .unwrap_err()
                .code,
            "product_artifact_name"
        );
    }
}
#[test]
fn terrain_tilelist_accepts_intrinsic_header_without_claiming_file_association() {
    let plan = Plan {
        components: Component::Terrain.bit(),
        db2: vec![],
        terrain: Selection::All,
        ..db2_plan()
    };
    let mut list = terrain::tilelist::TileList::new(plan.build).unwrap();
    list.set(7, 3).unwrap();
    let path = "maps/0042.tilelist";
    let bytes = list.bytes();
    Facts::default()
        .inspect(&plan, &artifact(Component::Terrain, path, bytes), bytes)
        .unwrap();
    // This intrinsic check deliberately does not prove the MAPS payload exists.
    assert!(terrain::tilelist::present(bytes, plan.build, 7, 3).unwrap());
}

#[test]
fn terrain_selection_rejects_off_plan_map_and_tilelist_artifacts() {
    for terrain in [
        Selection::Map(42),
        Selection::Tile {
            map: 43,
            x: 7,
            y: 3,
        },
    ] {
        let lab = Lab::new();
        let plan = Plan {
            components: Component::Terrain.bit(),
            db2: vec![],
            terrain,
            ..db2_plan()
        };
        let list = terrain::tilelist::TileList::new(plan.build).unwrap();
        lab.publish(
            context(&plan),
            &[(
                Component::Terrain,
                "maps/0043.tilelist".into(),
                list.bytes().to_vec(),
            )],
        );
        assert!(verify(&plan, &lab.output(), None).is_err());
    }
}

#[test]
fn navigation_selection_rejects_off_plan_global_map_artifact() {
    use forever_navigation::mmap;
    let plan = Plan {
        components: Component::Db2.bit()
            | Component::Terrain.bit()
            | Component::Collision.bit()
            | Component::Navigation.bit(),
        db2: vec![entry("LiquidType"), entry("Map")],
        terrain: Selection::All,
        navigation: Selection::Map(42),
        ..db2_plan()
    };
    plan.validate().unwrap();
    let bytes = mmap::encode_global(
        43,
        mmap::Params {
            origin: [0.0; 3],
            tile_width: 1.0,
            tile_height: 1.0,
            max_tiles: 1,
            max_polys: 1,
        },
        &[],
        Default::default(),
    )
    .unwrap();
    let path = mmap::global_path(43);
    assert!(
        Facts::default()
            .inspect(
                &plan,
                &artifact(Component::Navigation, &path, &bytes),
                &bytes
            )
            .is_err()
    );
}

#[test]
fn v2_verification_needs_no_keys_and_rejects_the_same_artifacts_under_v1_plan() {
    let lab = Lab::new();
    let original = db2_plan();
    let plan = Plan {
        decryption: crate::plan::Decryption::ProvidedSalsa20V1,
        ..original.clone()
    };
    let expected = lab.publish(
        context(&plan),
        &[db2_file("SpellCastTimes", empty_db2("SpellCastTimes"))],
    );
    assert_eq!(verify(&plan, &lab.output(), None).unwrap(), expected);
    assert_eq!(
        verify(&original, &lab.output(), None).unwrap_err().code,
        "product_plan_mismatch"
    );
}

#[test]
fn v3_verifier_enforces_whole_package_budget_without_scratch_or_keys() {
    use crate::plan::{CollisionStorage, DiskBudget};
    let mut plan = db2_plan();
    plan.components |= Component::Collision.bit();
    let budget = DiskBudget {
        raw_bytes: 32,
        wdt_bytes: 32,
        cache_bytes: 32,
        placement_bytes: 32,
        vmo_bytes: 32,
        map_bytes: 32,
        output_bytes: 1,
    };
    plan.collision_storage = CollisionStorage::LeasedDiskV1(budget);
    plan.validate().unwrap();
    let lab = Lab::new();
    lab.publish(
        context(&plan),
        &[db2_file("SpellCastTimes", empty_db2("SpellCastTimes"))],
    );
    assert_eq!(
        verify(&plan, &lab.output(), None).unwrap_err().code,
        "package_limit"
    );
}
