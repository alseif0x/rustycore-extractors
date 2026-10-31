# SPDX-License-Identifier: GPL-3.0-or-later
"""Build-70450 authenticated terrain with a declared partial Map contract."""
import json
from pathlib import Path
import unittest

ROOT = Path(__file__).resolve().parents[2]
CONTRACT = ROOT / "contracts/forever-product-contract-v25.json"
FIXTURES = ROOT / "tests/contracts/fixtures/forever-product-v25"


class ProductV25Contract(unittest.TestCase):
    def setUp(self):
        self.contract = json.loads(CONTRACT.read_bytes())

    def test_identity_and_closed_scope(self):
        c = self.contract
        self.assertEqual(
            (c["contract"], c["version"], c["publication_profile"], c["builds"]),
            ("forever-product-v25", 25, "Build70450AuthenticatedTerrainPartialMapV1", [70450]),
        )
        self.assertEqual((c["platform"], c["backend"], c["offline"]), ("linux", "rust-local", True))
        self.assertEqual(c["commands"], ["inspect", "extract", "resume", "verify", "verify-links"])
        plan = c["plan"]
        self.assertEqual(plan["schema_catalog"], "wowdbdefs-70450-v1")
        self.assertEqual(plan["component_mask"], 9)
        self.assertEqual(plan["components"], ["db2", "maps"])
        self.assertEqual(plan["decryption"], "disabled")
        self.assertEqual(plan["keys"], "refused")
        self.assertEqual(plan["partial_encrypted_sections"], "declared-skip")
        self.assertEqual(plan["db2_output_entries"], ["Map"])
        self.assertEqual(plan["terrain"], "all-available-maps")
        self.assertEqual(plan["terrain_coverage"], "maps/compatible-terrain-coverage-v3.tsv")
        self.assertEqual(plan["storage"], "resident")
        self.assertEqual(plan["navigation"], "none")
        self.assertEqual(plan["map_domain"], "none")
        self.assertEqual(plan["offmesh"], "disabled")
        self.assertEqual(plan["scratch"], "forbidden")
        self.assertEqual(c["package"]["limits_profile"], "forever-product-contract-v24")
        self.assertEqual(
            c["verification"]["profile"],
            "linux70450-authenticated-available-map-terrain-partial-map-product-v1",
        )
        for flag in (
            "full_extractor_complete",
            "runtime_claim",
            "format_parity_claim",
            "whole_client_map_coverage",
        ):
            self.assertFalse(c["verification"][flag])

    def test_plan_fixtures_are_canonical_and_require_the_declared_skip_gate(self):
        for filename, terrain in (("map2.tsv", "map:2"), ("all.tsv", "all-available-maps")):
            raw = (FIXTURES / filename).read_bytes()
            self.assertTrue(raw.endswith(b"\n"))
            self.assertNotIn(b"\r", raw)
            rows = [line.split("\t") for line in raw.decode("ascii").splitlines()]
            fields = {row[0]: row[1] for row in rows if row[0] != "db2"}
            self.assertEqual(fields["forever-product"], "25")
            self.assertEqual(fields["build"], "70450")
            self.assertEqual(fields["schema-catalog"], "wowdbdefs-70450-v1")
            self.assertEqual(fields["decryption"], "disabled")
            self.assertEqual(fields["partial_encrypted_sections"], "declared-skip")
            self.assertEqual(fields["components"], "db2,maps")
            self.assertEqual(fields["terrain"], terrain)
            self.assertEqual([row for row in rows if row[0] == "db2"], [["db2", "Map", "1349477"]])

    def test_unqualified_claims_and_acceptance_matrix_are_explicit(self):
        c = self.contract
        self.assertFalse(c["full_extractor_complete"])
        self.assertTrue(c["verification"]["partial_decryption_claim"] is False)
        cases = {case["case"]: case["expect"] for case in c["acceptance_matrix"]}
        self.assertIn("every E frame", cases["partial-map"])
        self.assertIn("undeclared partial data", cases["partial-verification"])
        self.assertIn("verify-links", cases["synthetic-map-and-terrain"])

    def test_real_acceptance_is_bounded_and_preserves_open_gates(self):
        c = self.contract
        cases = {case["case"]: case["expect"] for case in c["acceptance_matrix"]}
        self.assertIn("three artifacts / 15,344 bytes", cases["bounded-real-map48"])
        self.assertIn("3,395 artifacts / 215,670,284 bytes", cases["real-available-terrain"])
        self.assertIn("3,322 MAPS tiles, 71 tilelists", cases["real-available-terrain"])
        evidence = c["real_acceptance"]
        self.assertEqual(evidence["map48"]["files"], 3)
        self.assertEqual(evidence["map48"]["bytes"], 15344)
        self.assertEqual(evidence["all_available"]["files"], 3395)
        self.assertEqual(evidence["all_available"]["bytes"], 215670284)
        self.assertEqual(evidence["all_available"]["terrain_map_tiles"], 3322)
        self.assertEqual(evidence["all_available"]["terrain_tile_lists"], 71)
        self.assertEqual(evidence["partial_map_frames"], 3)
        self.assertEqual(evidence["partial_map_frame_bytes"], 575)
        self.assertFalse(evidence["network_attempted"])
        self.assertTrue(evidence["client_metadata_unchanged"])
        self.assertTrue(evidence["inputs_unchanged"])
        self.assertFalse(evidence["decrypted_sections"])
        self.assertNotIn("real_extraction", c["unqualified_status"])
        self.assertIn("whole-client extraction or acceptance", c["excluded"])
        self.assertFalse(c["unqualified_status"]["full_world_coverage"])
        self.assertFalse(c["unqualified_status"]["runtime_acceptance"])
        self.assertFalse(c["full_extractor_complete"])


if __name__ == "__main__":
    unittest.main()
