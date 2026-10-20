# SPDX-License-Identifier: GPL-3.0-or-later
"""Build-70450 selected-navigation contract and canonical synthetic plan."""
import json
from pathlib import Path
import unittest

ROOT = Path(__file__).resolve().parents[2]
CONTRACT = ROOT / "contracts/forever-product-contract-v23.json"
FIXTURE = ROOT / "tests/contracts/fixtures/forever-product-v23/map48.tsv"


class ProductV23Contract(unittest.TestCase):
    def setUp(self):
        self.contract = json.loads(CONTRACT.read_bytes())

    def test_profile_identity_and_narrow_cli_surface(self):
        contract = self.contract
        self.assertEqual(
            (
                contract["contract"],
                contract["version"],
                contract["publication_profile"],
                contract["builds"],
            ),
            ("forever-product-v23", 23, "Build70450SelectedNavigationV1", [70450]),
        )
        self.assertEqual(
            (contract["platform"], contract["backend"], contract["offline"]),
            ("linux", "rust-local", True),
        )
        self.assertEqual(
            contract["commands"], ["inspect", "extract", "resume", "verify", "verify-links"]
        )
        plan = contract["plan"]
        self.assertEqual((plan["components"], plan["component_mask"]), (["mmaps"], 32))
        self.assertEqual(plan["schema_catalog"], "wowdbdefs-70450-v1")
        self.assertEqual(plan["decryption"], "disabled")
        self.assertEqual(plan["keys"], "refused")
        self.assertEqual(plan["storage"], "resident")
        self.assertEqual(plan["navigation_selection"], "exactly one selected map")
        self.assertEqual(plan["terrain"], "none")
        self.assertEqual(plan["collision"], "none")
        self.assertEqual(plan["db2_output_entries"], 0)
        self.assertEqual(plan["source_db2_catalog_entries"], ["Map", "LiquidType"])
        self.assertFalse(plan["all_available_maps_selection"])
        self.assertFalse(plan["client_inspect"])
        self.assertTrue(plan["cli_inputs"]["extract_resume_require_input_pairs"])
        self.assertTrue(plan["cli_inputs"]["verify_links_requires_input_pairs"])
        self.assertFalse(plan["cli_inputs"]["verify_requires_client"])
        self.assertFalse(plan["cli_inputs"]["verify_links_requires_client"])

    def test_canonical_map48_plan_and_additive_profile_literals(self):
        raw = FIXTURE.read_bytes()
        self.assertTrue(raw.endswith(b"\n"))
        self.assertNotIn(b"\r", raw)
        pairs = [line.split("\t") for line in raw.decode("ascii").splitlines()]
        self.assertTrue(all(len(pair) == 2 for pair in pairs))
        self.assertEqual(
            dict(pairs),
            {
                "forever-product": "23",
                "build": "70450",
                "locale": "enUS",
                "index": "contiguous",
                "root": "header-50893-v2",
                "schema-catalog": "wowdbdefs-70450-v1",
                "decryption": "disabled",
                "components": "mmaps",
                "terrain": "none",
                "pack": "true",
                "navigation": "map:48",
                "big-base-unit": "false",
                "skip-liquids": "false",
                "maximum-slope": "55",
                "non-steep-slope": "55",
            },
        )
        plan_source = (ROOT / "crates/forever-extractor/src/plan.rs").read_text()
        publication = (ROOT / "crates/forever-extractor/src/plan/publication.rs").read_text()
        self.assertIn("Build70450SelectedNavigationV1", publication)
        self.assertIn("pub const fn is_v23", plan_source)
        self.assertIn("if self.is_v23()", plan_source)
        self.assertTrue(self.contract["legacy_preservation"].startswith("v20 terrain, v21 navigation, and v22"))

    def test_linked_sources_outputs_and_claim_boundaries(self):
        policy = self.contract["source_policy"]
        self.assertIn("available-section", policy["map"])
        self.assertIn("not a whole-file Map SHA", policy["map"])
        self.assertIn("complete bounded source read", policy["liquid_type"])
        self.assertIn("every map in the fresh Map active-ancestor closure", policy["terrain_input"])
        self.assertIn("known-present tile count known", policy["terrain_input"])
        self.assertIn("loaded WDT with known-present zero", policy["terrain_input"])
        self.assertIn("does not claim complete geometry", policy["vmap_input"])
        self.assertIn("may differ", policy["linked_contexts"])
        self.assertIn("no publication-profile field", policy["identity"])
        self.assertIn("exactly once", policy["locking_and_finalization"])
        self.assertIn("not an atomic multi-package snapshot", policy["locking_and_finalization"])
        self.assertIn("must not equal, contain, or be contained", policy["input_path_safety"])

        package = self.contract["package"]
        self.assertEqual(package["coverage"], "mmaps/selected-navigation-coverage-v1.tsv")
        self.assertEqual(package["global"], "mmaps/<map-width4>.mmap")
        self.assertIn("candidate tile", package["inventory"])
        self.assertNotIn("terrain tile", package["inventory"])
        self.assertIn("empty outcome is recorded in coverage", package["empty_tile"])
        self.assertTrue(package["provisional_until_sealed"])

        verification = self.contract["verification"]
        self.assertIn("no client reads", verification["verify_scope"])
        self.assertIn("both supplied v20 and v22", verification["verify_links_scope"])
        self.assertFalse(verification["current_source_snapshot"])
        self.assertFalse(verification["runtime_claim"])
        self.assertFalse(verification["format_parity_claim"])
        self.assertFalse(verification["atomic_snapshot_claim"])
        self.assertFalse(self.contract["coverage"]["complete_global_map_coverage_claim"])
        self.assertFalse(self.contract["coverage"]["complete_geometry_claim"])
        self.assertTrue(self.contract["coverage"]["empty_terrain_success_requires_loaded_known_zero"])
        self.assertFalse(self.contract["full_extractor_complete"])

    def test_limits_and_required_negative_acceptance_cases(self):
        limits = self.contract["limits"]
        self.assertEqual(limits["selected_map_closure_maps_max"], 4096)
        self.assertEqual(limits["selected_tiles_max"], 4096)
        self.assertEqual(limits["output_coverage_bytes_max"], 1024 * 1024)
        self.assertIn("no global cross-stage memory ledger", limits["meaning"])
        cases = {case["case"]: case["expect"] for case in self.contract["acceptance_matrix"]}
        for required in (
            "other-mask-build-or-selection",
            "keys-offmesh-or-scratch",
            "input-package-or-plan-mismatch",
            "incomplete-terrain-coverage",
            "known-empty-terrain",
            "vmap-closure-or-source-drift",
            "output-path-overlap",
            "nested-callback-invariant",
            "map-or-liquid-source-drift",
            "empty-selected-tile",
            "offline-verify",
            "offline-verify-links",
            "non-linux-execution",
        ):
            self.assertIn(required, cases)
            self.assertTrue(cases[required])
        for excluded in (
            "terrain or VMap generation inside the v23 operation",
            "off-mesh connections",
            "global process RSS or cross-stage memory guarantee",
            "Windows execution",
        ):
            self.assertIn(excluded, self.contract["excluded"])


if __name__ == "__main__":
    unittest.main()
