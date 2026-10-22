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


if __name__ == "__main__":
    unittest.main()
