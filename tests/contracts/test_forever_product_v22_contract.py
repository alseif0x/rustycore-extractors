# SPDX-License-Identifier: GPL-3.0-or-later
"""Build-70450 selected-map geometry contract and canonical synthetic plan."""
import json
from pathlib import Path
import unittest

ROOT = Path(__file__).resolve().parents[2]
CONTRACT = ROOT / "contracts/forever-product-contract-v22.json"
FIXTURE = ROOT / "tests/contracts/fixtures/forever-product-v22/map2.tsv"


class ProductV22Contract(unittest.TestCase):
    def setUp(self):
        self.contract = json.loads(CONTRACT.read_bytes())

    def test_profile_identity_mask_key_policy_and_cli_surface(self):
        contract = self.contract
        self.assertEqual(
            (
                contract["contract"],
                contract["version"],
                contract["publication_profile"],
                contract["builds"],
            ),
            ("forever-product-v22", 22, "Build70450SelectedGeometryV1", [70450]),
        )
        self.assertEqual(
            (contract["platform"], contract["backend"], contract["offline"]),
            ("linux", "rust-local", True),
        )
        self.assertEqual(
            contract["commands"], ["inspect", "extract", "resume", "verify", "verify-links"]
        )
        plan = contract["plan"]
        self.assertEqual(plan["schema_catalog"], "wowdbdefs-70450-v1")
        self.assertEqual((plan["components"], plan["component_mask"]), (["vmaps"], 16))
        self.assertEqual(plan["decryption"], "disabled")
        self.assertEqual(plan["keys"], "refused")
        self.assertEqual(plan["storage"], "linux-leased-append-disk-world-v1")
        self.assertTrue(plan["extract_resume_require_scratch"])
        self.assertFalse(plan["all_available_maps_selection"])
        self.assertFalse(plan["inspect_requires_client"])
        self.assertEqual(plan["collision_domain"], "map:<canonical-u32>")
        self.assertEqual(plan["gameobject_scope"], "excluded-by-profile-v1")
        self.assertEqual(plan["model_scope"], "placement-only-v1")
        self.assertEqual(plan["terrain"], "none")
        self.assertEqual(plan["navigation"], "none")
        self.assertEqual(contract["coverage"]["profile"], plan["coverage_profile"])
        self.assertFalse(contract["coverage"]["complete_global_map_coverage_claim"])
        self.assertFalse(contract["full_extractor_complete"])

    def test_canonical_plan_fixture_and_own_profile_literals(self):
        raw = FIXTURE.read_bytes()
        self.assertTrue(raw.endswith(b"\n"))
        self.assertNotIn(b"\r", raw)
        pairs = [line.split("\t") for line in raw.decode("ascii").splitlines()]
        self.assertTrue(all(len(pair) == 2 for pair in pairs))
        fields = dict(pairs)
        self.assertEqual(
            fields,
            {
                "forever-product": "22",
                "build": "70450",
                "locale": "enUS",
                "index": "contiguous",
                "root": "header-50893-v2",
                "schema-catalog": "wowdbdefs-70450-v1",
                "decryption": "disabled",
                "collision-storage": "linux-leased-append-disk-world-v1",
                "collision-raw-bytes": "1048576",
                "collision-wdt-bytes": "1048576",
                "collision-cache-bytes": "1048576",
                "collision-placement-bytes": "1048576",
                "collision-vmo-bytes": "1048576",
                "collision-map-bytes": "1048576",
                "collision-output-bytes": "1048576",
                "collision-profile": "linux70450-selected-map-geometry-vmap-product-v1",
                "collision-domain": "map:2",
                "collision-coverage": "linux70450-selected-map-geometry-coverage-v1",
                "gameobject-scope": "excluded-by-profile-v1",
                "model-scope": "placement-only-v1",
                "components": "vmaps",
                "terrain": "none",
                "pack": "true",
                "navigation": "none",
                "big-base-unit": "false",
                "skip-liquids": "false",
                "maximum-slope": "55",
                "non-steep-slope": "55",
            },
        )
        plan_tests = (
            ROOT / "crates/forever-extractor/src/plan/v22_tests.rs"
        ).read_text()
        for expected in (
            "v22_refuses_masks_keys_and_non_map_domains_before_io",
            "Decryption::ProvidedSalsa20V1",
            '"all-available-maps"',
            '"map:02"',
            "product_plan_component_scope",
        ):
            self.assertIn(expected, plan_tests)

    def test_declared_omissions_verification_and_unqualified_status(self):
        contract = self.contract
        policy = contract["source_policy"]
        self.assertIn("available-section", policy["map"])
        self.assertIn("active ancestor closure", policy["map"])
        self.assertIn("excluded by profile policy", policy["gameobjects"])
        self.assertIn("not an observed source failure or cause", policy["gameobjects"])
        self.assertIn("actual selected-domain placement requests", policy["models"])
        self.assertIn("GODI startup_facts().reads is zero", policy["models"])
        self.assertIn("exactly matches used_models", policy["models"])
        self.assertIn("LiquidMaterial", policy["liquids"])
        self.assertIn("LiquidType", policy["liquids"])
        self.assertIn("LiquidObject is out of scope", policy["liquids"])
        self.assertIn("not requested", policy["liquids"])
        self.assertTrue(
            any("request_made=false" in fact for fact in contract["coverage"]["facts"])
        )
        self.assertTrue(
            any("startup reads zero" in fact for fact in contract["coverage"]["facts"])
        )
        self.assertFalse(contract["verification"]["runtime_claim"])
        self.assertFalse(contract["verification"]["format_parity_claim"])
        self.assertFalse(contract["unqualified_status"]["real_extraction"])
        self.assertFalse(contract["unqualified_status"]["runtime_acceptance"])
        self.assertFalse(contract["unqualified_status"]["full_world_coverage"])
        self.assertFalse(contract["unqualified_status"]["format_parity"])
        self.assertFalse(contract["unqualified_status"]["mmaps"])
        for excluded in ("MMaps", "LiquidObject", "Windows execution"):
            self.assertIn(excluded, contract["excluded"])
        self.assertTrue(
            any("full-world coverage" in value for value in contract["excluded"])
        )

        plan_source = (ROOT / "crates/forever-extractor/src/plan.rs").read_text()
        publication = (
            ROOT / "crates/forever-extractor/src/plan/publication.rs"
        ).read_text()
        verify = (ROOT / "crates/forever-extractor/src/verify.rs").read_text()
        driver = (ROOT / "crates/forever-extractor/src/driver.rs").read_text()
        self.assertIn("Build70450SelectedGeometryV1", publication)
        self.assertIn("pub const fn is_v22", plan_source)
        self.assertIn("if plan.is_v22()", verify)
        self.assertIn("collision_geometry_v22::extract", driver)


if __name__ == "__main__":
    unittest.main()
