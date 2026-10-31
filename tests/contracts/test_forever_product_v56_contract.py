# SPDX-License-Identifier: GPL-3.0-or-later
import json
from pathlib import Path
import unittest

ROOT = Path(__file__).resolve().parents[2]


class ProductV56Contract(unittest.TestCase):
    def setUp(self):
        self.contract = json.loads((ROOT / "contracts/forever-product-contract-v56.json").read_text())

    def test_canonical_plan_identity_and_scope(self):
        text = (ROOT / "tests/contracts/fixtures/forever-product-v56/all.tsv").read_text()
        rows = dict(line.split("\t") for line in text.splitlines())
        self.assertEqual(len(rows), 15)
        self.assertEqual(rows["forever-product"], "56")
        self.assertEqual(rows["components"], "mmaps")
        self.assertEqual(rows["navigation"], "all-available-maps")
        self.assertEqual(rows["terrain"], "none")
        self.assertEqual(rows["decryption"], "disabled")
        self.assertTrue(text.endswith("\n"))
        self.assertEqual(self.contract["components"], 32)

    def test_whole_sealed_source_and_offline_interface(self):
        source = self.contract["source"]
        self.assertEqual(source["contract"], "forever-product-v51")
        self.assertEqual(source["components"], 25)
        self.assertEqual((source["package_count"], source["plan_count"]), (1, 1))
        self.assertFalse(source["client_reads"])
        self.assertEqual(source["v25_v50_or_mixed_packages"], "reject")
        self.assertEqual(source["cli_package_option"], "--terrain-package")
        self.assertEqual(source["cli_plan_option"], "--terrain-plan")
        self.assertFalse(self.contract["verification"]["client_reads"])

    def test_limits_outcomes_and_no_claim_transfer(self):
        resources = self.contract["resources"]
        self.assertEqual(resources["maps_max"], 4096)
        self.assertEqual(resources["tiles_max"], 4096 * 4096)
        self.assertEqual(resources["vertices_max"], 1_000_000)
        self.assertEqual(resources["triangles_max"], 500_000)
        self.assertEqual(resources["mesh_heap_bytes_max"], 48 * 1024 * 1024)
        self.assertEqual(resources["geometry_v51_budgets"], "unchanged")
        self.assertEqual(self.contract["execution"]["offmesh_connections"], 0)
        self.assertFalse(self.contract["execution"]["rebuild_maps_vmaps"])
        self.assertEqual(self.contract["execution"]["partial_failure"], "fatal-no-final-package")
        self.assertIn("native-negative-ntris", self.contract["execution"]["partial_triangulation_policy"])
        self.assertIn("last-visited-node", self.contract["execution"]["detail_center_fallback_policy"])
        self.assertIn("RC_UNSET_HEIGHT", self.contract["execution"]["unset_detail_height_policy"])
        self.assertIn("partial-triangulations-per-map", self.contract["output"]["facts"])
        self.assertIn("detail-center-fallbacks-per-map", self.contract["output"]["facts"])
        self.assertIn("unset-height-samples-per-map", self.contract["output"]["facts"])
        self.assertTrue(all(value is False for value in self.contract["claims"].values()))
        self.assertIn("persistent", self.contract["real_acceptance"])
        self.assertEqual(self.contract["output"]["format"], "MMap16")
        self.assertTrue((ROOT / "docs/forever-product-v56.md").is_file())


if __name__ == "__main__":
    unittest.main()
