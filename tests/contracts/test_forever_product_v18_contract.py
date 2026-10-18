# SPDX-License-Identifier: GPL-3.0-or-later
"""Explicit keyed 70450 compatible collision product facts; synthetic metadata only."""
from pathlib import Path
import json
import unittest


ROOT = Path(__file__).resolve().parents[2]


class ProductV18Contract(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.contract = json.loads(
            (ROOT / "contracts/forever-product-contract-v18.json").read_bytes()
        )

    def test_identity_and_exact_collision_key_policy(self):
        contract = self.contract
        self.assertEqual(
            (
                contract["version"],
                contract["contract"],
                contract["publication_profile"],
            ),
            (18, "forever-product-v18", "Build70450CollisionProvidedKeysV1"),
        )
        self.assertEqual(contract["builds"], [70450])
        self.assertEqual(contract["platform"], "linux")
        self.assertEqual(contract["backend"], "rust-local")
        self.assertTrue(contract["offline"])
        self.assertEqual(
            contract["commands"],
            ["inspect", "extract", "resume", "verify", "verify-links"],
        )
        plan = contract["plan"]
        self.assertEqual(plan["schema_catalog"], "wowdbdefs-70450-v1")
        self.assertEqual(plan["components"], ["vmaps"])
        self.assertEqual(plan["component_mask"], 16)
        self.assertEqual(plan["collision_storage"], "linux-leased-append-disk-world-v1")
        self.assertEqual(plan["default_budget_bytes_per_key"], 1_048_576)
        self.assertEqual(
            plan["collision_budget_keys"],
            [
                "collision-raw-bytes",
                "collision-wdt-bytes",
                "collision-cache-bytes",
                "collision-placement-bytes",
                "collision-vmo-bytes",
                "collision-map-bytes",
                "collision-output-bytes",
            ],
        )
        self.assertEqual(
            plan["collision_profile"], "linux70450-compatible-gnu15-vmap-product-v1"
        )
        self.assertEqual(plan["collision_domain"], ["all-available-maps", "map:<canonical-u32>"])
        self.assertEqual(plan["collision_coverage"], "compatible-collision-coverage-v1")
        self.assertEqual(plan["decryption"], "provided-salsa20-v1")
        self.assertEqual(plan["terrain"], "none")
        self.assertEqual(plan["navigation"], "none")
        self.assertEqual(plan["offmesh"], "field absent")
        self.assertEqual(plan["db2_output_entries"], 0)
        self.assertTrue(plan["pack"])
        self.assertEqual(plan["maximum_slope"], 55)
        self.assertEqual(plan["non_steep_slope"], 55)
        self.assertFalse(contract["full_extractor_complete"])

    def test_v18_plan_is_v15_collision_plan_with_only_revision_and_key_policy_changed(self):
        v15 = (
            ROOT / "tests/contracts/fixtures/forever-product-v15/all-available-maps.tsv"
        ).read_text()
        v18 = (
            ROOT / "tests/contracts/fixtures/forever-product-v18/all-available-maps.tsv"
        ).read_text()
        expected = v15.replace("forever-product\t15\n", "forever-product\t18\n", 1).replace(
            "decryption\tdisabled\n", "decryption\tprovided-salsa20-v1\n", 1
        )
        self.assertEqual(v18, expected)
        fields = [line.split("\t", 1)[0] for line in v18.splitlines()]
        self.assertEqual(fields[0], "forever-product")
        self.assertEqual(fields[1], "build")
        self.assertNotIn("db2", fields)
        self.assertNotIn("offmesh", fields)
        self.assertTrue(v18.endswith("\n"))

    def test_key_policy_and_partial_matches_do_not_claim_complete_decryption(self):
        keys = self.contract["provided_keys"]
        self.assertEqual(keys["option"], "--keys PATH")
        self.assertEqual(
            keys["required_for"], ["inspect --client", "extract", "resume"]
        )
        self.assertEqual(keys["forbidden_for"], ["verify", "verify-links", "inspect without --client"])
        self.assertEqual(keys["max_file_bytes"], 262144)
        self.assertEqual(keys["max_entries"], 4096)
        self.assertIn("sorted unique lowercase hex16", keys["record_format"])
        self.assertFalse(keys["discovery"])
        self.assertFalse(keys["built_in_keys"])
        self.assertFalse(keys["network"])
        self.assertIn("explicit regular file", keys["input"])
        self.assertIn("public key list or a partial match never proves complete decryption", self.contract["verification"]["key_coverage"])
        self.assertFalse(self.contract["coverage"]["complete_global_map_coverage_claim"])

    def test_revisions_14_through_17_and_generic_admission_remain_distinct(self):
        v14 = json.loads((ROOT / "contracts/forever-product-contract-v14.json").read_bytes())
        v15 = json.loads((ROOT / "contracts/forever-product-contract-v15.json").read_bytes())
        v16 = json.loads((ROOT / "contracts/forever-product-contract-v16.json").read_bytes())
        v17 = json.loads((ROOT / "contracts/forever-product-contract-v17.json").read_bytes())
        self.assertEqual((v14["version"], v14["builds"]), (14, [70425]))
        self.assertEqual(v15["publication_profile"], "Build70450CompatibleCollisionV1")
        self.assertEqual(v15["plan"]["decryption"], "disabled")
        self.assertEqual(v16["publication_profile"], "Build70450Db2OnlyV1")
        self.assertEqual(v17["publication_profile"], "Build70450Db2ProvidedKeysV1")
        self.assertEqual(v17["plan"]["components"], ["db2"])
        self.assertTrue(v17["provided_keys"]["network"] is False)
        self.assertEqual(
            self.contract["legacy_preservation"]["v15"],
            "Build70450CompatibleCollisionV1; build70450 compatible GNU15-v2; disabled keys",
        )
        self.assertEqual(
            self.contract["legacy_preservation"]["v16"],
            "Build70450Db2OnlyV1; build70450 DB2-only; disabled keys",
        )
        self.assertEqual(
            self.contract["legacy_preservation"]["v17"],
            "Build70450Db2ProvidedKeysV1; build70450 DB2-only; provided-salsa20-v1",
        )
        admission = (ROOT / "crates/forever-casc/src/admission.rs").read_text()
        supported = admission.split("pub const SUPPORTED_CLIENTS:", 1)[1].split("];", 1)[0]
        self.assertNotIn("70450", supported)
        self.assertTrue(self.contract["legacy_preservation"]["supported_clients"])

    def test_keyed_store_compatible_verify_replay_and_identity_projection_routes(self):
        store = (ROOT / "crates/forever-casc/src/local_store.rs").read_text()
        driver = (ROOT / "crates/forever-extractor/src/driver.rs").read_text()
        verify = (ROOT / "crates/forever-extractor/src/verify.rs").read_text()
        execution = (ROOT / "crates/forever-extractor/src/driver/execution.rs").read_text()
        self.assertIn("open_db2_70450_with_keys", store)
        self.assertIn("admit_db2_70450", driver)
        self.assertIn("open_db2_70450_with_keys", driver)
        collision_dispatch = driver.split("fn extract_collision(", 1)[1].split("\n    }\n}", 1)[0]
        self.assertIn(
            'ifplan.is_v15()||plan.is_v18(){returnself.preflight.as_ref().ok_or("product_scratch_required")?.run_compatible_70450(store,run,plan.map_domain,self.context);}',
            "".join(collision_dispatch.split()),
        )
        self.assertIn("plan.is_v14() || plan.is_v15() || plan.is_v18()", verify)
        self.assertIn("plan.is_v16() || plan.is_v17()", verify)
        self.assertNotIn("v17_metadata_plan", verify)
        self.assertIn("plan.is_v18()", execution)
        self.assertIn("v18_collision_resume_replay_mask_clears_only_the_sealed_collision_component", execution)

    def test_linux_ci_and_readme_enumerate_v18(self):
        workflow = json.loads((ROOT / ".github/workflows/forever-rust.yml").read_bytes())
        step = next(
            item
            for item in workflow["jobs"]["linux"]["steps"]
            if item.get("name") == "Check product v18 contract and synthetic CLI"
        )
        self.assertIn(
            "python -m unittest discover -s tests/contracts -p test_forever_product_v18_contract.py -v",
            step["run"].splitlines(),
        )
        self.assertIn(
            "cargo test --offline --locked -p forever-extractor --test product_v18_cli --no-default-features -- --test-threads=1",
            step["run"].splitlines(),
        )
        readme = (ROOT / "README.md").read_text()
        self.assertIn("Product v18 separately admits", readme)
        self.assertIn("docs/forever-product-v18.md", readme)


if __name__ == "__main__":
    unittest.main()
