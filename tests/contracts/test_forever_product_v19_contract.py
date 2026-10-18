# SPDX-License-Identifier: GPL-3.0-or-later
"""Explicit Linux build-70450 DB2+GT+cameras product facts; synthetic only."""
from pathlib import Path
import hashlib
import json
import unittest


ROOT = Path(__file__).resolve().parents[2]


def load(name):
    return json.loads((ROOT / "contracts" / name).read_bytes())


class ProductV19Contract(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.contract = load("forever-product-contract-v19.json")

    def test_exact_identity_scope_camera_dependency_and_linux_only_profile(self):
        c = self.contract
        self.assertEqual(
            (c["version"], c["contract"], c["publication_profile"]),
            (19, "forever-product-v19", "Build70450Db2GtCamerasV1"),
        )
        self.assertEqual(c["builds"], [70450])
        self.assertEqual((c["platform"], c["backend"]), ("linux", "rust-local"))
        self.assertTrue(c["offline"])
        self.assertEqual(c["commands"], ["inspect", "extract", "resume", "verify", "verify-links"])
        plan = c["plan"]
        self.assertEqual(plan["schema_catalog"]["id"], "wowdbdefs-70450-v1")
        self.assertEqual(plan["schema_catalog"]["required_build"], 70450)
        self.assertEqual(plan["components"], ["db2", "gt", "cameras"])
        self.assertEqual(plan["component_mask"], 7)
        self.assertEqual(plan["collision_storage"], "resident")
        self.assertEqual(plan["map_domain"], "field absent")
        self.assertEqual(plan["terrain"], ["none"])
        self.assertEqual(plan["navigation"], ["none"])
        self.assertIn("Field must be absent", plan["offmesh_connections"])
        self.assertEqual(plan["decryption"], "required after schema-catalog; disabled only")
        self.assertEqual(plan["dependencies"], {"cameras": ["db2", "CinematicCamera"]})
        self.assertFalse(c["full_extractor_complete"])

    def test_canonical_fixture_and_cinematiccamera_catalog_facts(self):
        raw = (ROOT / "contracts/wowdbdefs-70450-v1.json").read_bytes()
        catalog = json.loads(raw)
        camera = next(row for row in catalog["tables"] if row["name"] == "CinematicCamera")
        declared = self.contract["fixed_components"]["cameras"]
        self.assertEqual(hashlib.sha256(raw).hexdigest(), self.contract["plan"]["schema_catalog"]["facts_sha256"])
        self.assertEqual(declared["schema_catalog"], self.contract["plan"]["schema_catalog"]["id"])
        for field in ("file_data_id", "table_hash", "layout_hash", "file_field_count"):
            self.assertEqual(declared[field], camera[field])
        self.assertEqual(declared["file_id_field"], 3)
        self.assertEqual(camera["file_data_id"], 1_294_214)

        fixture = (
            ROOT / "tests/contracts/fixtures/forever-product-v19/disabled.tsv"
        ).read_text()
        self.assertTrue(fixture.endswith("\n"))
        self.assertTrue(fixture.startswith("forever-product\t19\nbuild\t70450\n"))
        self.assertIn("schema-catalog\twowdbdefs-70450-v1\n", fixture)
        self.assertIn("decryption\tdisabled\n", fixture)
        self.assertIn("components\tdb2,gt,cameras\n", fixture)
        self.assertNotIn("collision-storage", fixture)
        self.assertNotIn("collision-domain", fixture)
        self.assertNotIn("offmesh\t", fixture)
        self.assertEqual(
            [line.split("\t") for line in fixture.splitlines() if line.startswith("db2\t")],
            [["db2", "CinematicCamera", "1294214"]],
        )
        self.assertEqual(
            [
                line.split("\t", 1)[0]
                for line in fixture.splitlines()
                if not line.startswith("db2\t")
            ],
            self.contract["plan"]["ordered_keys"],
        )

    def test_disabled_keys_and_prior_target_contracts_remain_distinct(self):
        self.assertEqual(self.contract["key_policy"]["decryption"], "disabled")
        self.assertEqual(self.contract["key_policy"]["failure_code"], "product_keys_forbidden")
        self.assertFalse(self.contract["key_policy"]["key_material_in_outputs"])
        v14 = load("forever-product-contract-v14.json")
        v15 = load("forever-product-contract-v15.json")
        v16 = load("forever-product-contract-v16.json")
        v17 = load("forever-product-contract-v17.json")
        v18 = load("forever-product-contract-v18.json")
        self.assertEqual(v14["publication_profile"], "Build70425CompatibleCollisionV1")
        self.assertEqual(v15["publication_profile"], "Build70450CompatibleCollisionV1")
        self.assertEqual(v15["plan"]["decryption"], "disabled")
        self.assertEqual(v16["publication_profile"], "Build70450Db2OnlyV1")
        self.assertEqual(v17["publication_profile"], "Build70450Db2ProvidedKeysV1")
        self.assertEqual(v18["publication_profile"], "Build70450CollisionProvidedKeysV1")
        self.assertEqual(
            self.contract["legacy_preservation"]["canonical_versions"],
            [1, 2, 3, 4, 5, 6, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19],
        )
        self.assertTrue(self.contract["legacy_preservation"]["catalog_does_not_select_v19"])
        admission = (ROOT / "crates/forever-casc/src/admission.rs").read_text()
        supported = admission.split("pub const SUPPORTED_CLIENTS:", 1)[1].split("];", 1)[0]
        self.assertNotIn("70450", supported)

    def test_v19_is_pinned_to_the_same_gt_camera_route_as_v11(self):
        plan = (ROOT / "crates/forever-extractor/src/plan.rs").read_text()
        plan_compact = "".join(plan.split())
        self.assertIn(
            "pubconstfnis_gt_cameras(&self)->bool{self.is_v9()||self.is_v11()||self.is_v19()}",
            plan_compact,
        )

        driver = (ROOT / "crates/forever-extractor/src/driver.rs").read_text()
        db2_route = driver.split("fn db2(", 1)[1].split("fn game_tables(", 1)[0]
        self.assertIn("letcap=db2_read_limit(plan,total)?", "".join(db2_route.split()))
        db2_cap = driver.split("fn db2_read_limit(", 1)[1].split("fn camera_remaining(", 1)[0]
        self.assertIn(
            "ifplan.is_gt_cameras()||plan.is_v10()||plan.is_v16()||plan.is_v17()||plan.is_available_terrain()",
            "".join(db2_cap.split()),
        )
        game_tables = driver.split("fn game_tables(", 1)[1].split("fn cameras(", 1)[0]
        self.assertIn("for table in gt::Table::ALL", game_tables)
        self.assertIn("gt::parse_numeric(&bytes,table)", "".join(game_tables.split()))
        cameras = driver.split("fn cameras(", 1)[1].split("fn camera_plan(", 1)[0]
        self.assertIn("letlimit=camera_read_limit(product,total)?", "".join(cameras.split()))
        camera_cap = driver.split("fn camera_read_limit(", 1)[1].split("structTerrainSink", 1)[0]
        self.assertIn("ifplan.is_gt_cameras(){camera_remaining(total)}", "".join(camera_cap.split()))
        camera_plan = driver.split("fn camera_plan(", 1)[1].split("fn db2_remaining(", 1)[0]
        self.assertIn("cameras::discover(source)?", "".join(camera_plan.split()))
        self.assertIn("v19_synthetic_numeric_rows_use_the_catalog_parsers", driver)
        self.assertIn("v19_camera_plan_uses_shared_reader_with_a_synthetic_sealed_source", driver)
        self.assertIn("v19_uses_the_v11_cumulative_db2_and_camera_read_caps", driver)

        execution = (ROOT / "crates/forever-extractor/src/driver/execution.rs").read_text()
        compact_execution = "".join(execution.split())
        self.assertIn("Component::GameTables=>work.game_tables()", compact_execution)
        self.assertIn("Component::Cameras=>work.cameras()", compact_execution)
        verify = (ROOT / "crates/forever-extractor/src/verify.rs").read_text()
        compact_verify = "".join(verify.split())
        self.assertIn("ifuses_target_metadata_preflight(plan){package::verify_with_preflight(", compact_verify)
        self.assertIn(
            "fnuses_target_metadata_preflight(plan:&Plan)->bool{plan.is_gt_cameras()||plan.is_v10()||plan.is_v16()||plan.is_v17()||plan.is_available_terrain()}",
            compact_verify,
        )
        self.assertIn("v9::admit_metadata(plan,artifacts)", compact_verify)
        self.assertIn("v19_uses_the_same_gt_camera_metadata_preflight_as_v11", verify)

    def test_70425_and_70450_context_sets_are_explicit_and_exhaustive(self):
        model = (ROOT / "crates/forever-casc/src/package/model.rs").read_text()
        self.assertIn("70425 => self.components == Component::Collision.bit()", model)
        self.assertIn("context70425_is_exact_linux_collision_mask_only", model)
        self.assertIn("context70450_admits_only_the_three_linux_product_masks", model)
        self.assertIn(
            "Component::Db2.bit() | Component::GameTables.bit() | Component::Cameras.bit()",
            model,
        )

    def test_linux_ci_and_readme_enumerate_v19(self):
        workflow = json.loads((ROOT / ".github/workflows/forever-rust.yml").read_bytes())
        step = next(
            item
            for item in workflow["jobs"]["linux"]["steps"]
            if item.get("name") == "Check product v19 contract and synthetic CLI"
        )
        self.assertIn(
            "python -m unittest discover -s tests/contracts -p test_forever_product_v19_contract.py -v",
            step["run"].splitlines(),
        )
        self.assertIn(
            "cargo test --offline --locked -p forever-extractor --test product_v19_cli --no-default-features -- --test-threads=1",
            step["run"].splitlines(),
        )
        readme = (ROOT / "README.md").read_text()
        self.assertIn("Product v19 separately admits", readme)
        self.assertIn("docs/forever-product-v19.md", readme)


if __name__ == "__main__":
    unittest.main()
