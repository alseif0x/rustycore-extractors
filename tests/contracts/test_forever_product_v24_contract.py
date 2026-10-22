# SPDX-License-Identifier: GPL-3.0-or-later
"""Build-70450 sealed-Map authenticated-terrain contract and plan fixtures."""
import json
from pathlib import Path
import unittest

ROOT = Path(__file__).resolve().parents[2]
CONTRACT = ROOT / "contracts/forever-product-contract-v24.json"
FIXTURES = ROOT / "tests/contracts/fixtures/forever-product-v24"


class ProductV24Contract(unittest.TestCase):
    def setUp(self):
        self.contract = json.loads(CONTRACT.read_bytes())

    def test_profile_identity_and_closed_scope(self):
        c = self.contract
        self.assertEqual(
            (c["contract"], c["version"], c["publication_profile"], c["builds"]),
            ("forever-product-v24", 24, "Build70450AuthenticatedTerrainV1", [70450]),
        )
        self.assertEqual((c["platform"], c["backend"], c["offline"]), ("linux", "rust-local", True))
        self.assertEqual(c["commands"], ["inspect", "extract", "resume", "verify", "verify-links"])

        plan = c["plan"]
        self.assertEqual(plan["schema_catalog"], "wowdbdefs-70450-v1")
        self.assertEqual(plan["component_mask"], 9)
        self.assertEqual(plan["components"], ["db2", "maps"])
        self.assertEqual(plan["component_bits"], {"db2": 1, "gt": 2, "cameras": 4, "maps": 8})
        self.assertEqual(plan["db2_output_entries"], ["Map"])
        self.assertEqual(plan["storage"], "resident")
        self.assertEqual(plan["terrain_selection"], ["all-available-maps", "map:<canonical-u32>"])
        self.assertEqual(plan["navigation"], "none")
        self.assertEqual(plan["decryption"], "disabled")
        self.assertEqual(plan["keys"], "refused")

    def test_canonical_v24_plans_and_v3_identity_boundary(self):
        for name, terrain in (("map2.tsv", "map:2"), ("all.tsv", "all-available-maps")):
            raw = (FIXTURES / name).read_bytes()
            self.assertTrue(raw.endswith(b"\n"))
            self.assertNotIn(b"\r", raw)
            records = [line.split("\t") for line in raw.decode("ascii").splitlines()]
            pairs = [record for record in records if record[0] != "db2"]
            self.assertTrue(all(len(pair) == 2 for pair in pairs))
            self.assertEqual([record for record in records if record[0] == "db2"], [["db2", "Map", "1349477"]])
            fields = dict(pairs)
            self.assertEqual(fields["forever-product"], "24")
            self.assertEqual(fields["build"], "70450")
            self.assertEqual(fields["schema-catalog"], "wowdbdefs-70450-v1")
            self.assertEqual(fields["components"], "db2,maps")
            self.assertEqual(fields["terrain"], terrain)

        self.assertEqual(
            self.contract["package"]["terrain_coverage"],
            "maps/compatible-terrain-coverage-v3.tsv",
        )
        identity = self.contract["source_policy"]["identity_boundary"]
        for fact in ("read_map_70450().materialize()", "Map ID, WDT FileDataID", "materialized counts"):
            self.assertIn(fact, identity)
        verification = self.contract["verification"]
        self.assertEqual(verification["coverage_authentication"], "independently_derived")
        self.assertIn("WDT FileDataID", verification["verify_scope"])
        self.assertFalse(verification["whole_client_map_coverage"])
        self.assertFalse(verification["runtime_claim"])
        self.assertFalse(verification["format_parity_claim"])
        self.assertFalse(self.contract["full_extractor_complete"])

    def test_v20_mask_8_is_not_widened_by_v24_mask_9(self):
        v20 = json.loads((ROOT / "contracts/forever-product-contract-v20.json").read_bytes())
        self.assertEqual(v20["plan"]["component_mask"], 8)
        self.assertEqual(v20["package"]["coverage"], "maps/compatible-terrain-coverage-v2.tsv")
        self.assertEqual(self.contract["plan"]["component_mask"], 9)
        cases = {case["case"]: case["expect"] for case in self.contract["acceptance_matrix"]}
        self.assertIn("only Db2|Terrain=9 for v24", cases["component-mask"])
        self.assertIn("v20 terrain profile remains mask 8", cases["component-mask"])
        self.assertIn("WDT FileDataIDs", cases["sealed-map-identity"])
        self.assertIn("extract/resume", cases["synthetic-map-and-terrain"])
        self.assertIn("tampering rejection", cases["synthetic-map-and-terrain"])


if __name__ == "__main__":
    unittest.main()
