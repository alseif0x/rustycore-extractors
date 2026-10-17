# SPDX-License-Identifier: GPL-3.0-or-later
"""Public terrain capability and authored plan examples; no client reads."""
import json
import importlib.util
from pathlib import Path
import unittest

ROOT = Path(__file__).resolve().parents[2]


class ProductV13Contract(unittest.TestCase):
    def setUp(self):
        self.contract = json.loads(
            (ROOT / "contracts/forever-product-contract-v13.json").read_bytes())

    def test_explicit_terrain_scope(self):
        c = self.contract
        self.assertEqual((c["version"], c["builds"], c["platform"], c["backend"]),
                         (13, [70374], "linux", "rust-local"))
        self.assertEqual(c["plan"]["component_mask"], 8)
        self.assertEqual(c["plan"]["components"], ["maps"])
        self.assertEqual(c["plan"]["decryption"], "disabled")
        self.assertFalse(c["plan"]["tile_subsets"])
        self.assertFalse(c["full_extractor_complete"])
        self.assertFalse(c["source_policy"]["native_failure_parity"])

    def test_canonical_examples(self):
        for name, selection in (("map0", "map:0"),
                                ("available", "all-available-maps")):
            raw = (ROOT / f"tests/contracts/fixtures/forever-product-v13/{name}.tsv").read_bytes()
            self.assertTrue(raw.endswith(b"\n"))
            self.assertNotIn(b"\r", raw)
            pairs = [line.split("\t") for line in raw.decode("ascii").splitlines()]
            self.assertEqual(len(pairs), 15)
            self.assertTrue(all(len(p) == 2 for p in pairs))
            fields = dict(pairs)
            self.assertEqual(len(fields), 15)
            self.assertEqual(fields["forever-product"], "13")
            self.assertEqual(fields["build"], "70374")
            self.assertEqual(fields["terrain"], selection)
            self.assertEqual(fields["components"], "maps")
            self.assertEqual(fields["decryption"], "disabled")
            self.assertEqual(fields["navigation"], "none")

    def test_coverage_and_verification_profiles(self):
        c = self.contract
        source = (ROOT / "crates/forever-extractor/src/driver/terrain_v13.rs").read_text()
        self.assertIn(c["package"]["coverage"], source)
        plan = (ROOT / "crates/forever-extractor/src/plan.rs").read_text()
        self.assertIn(c["verification"]["profile"], plan)
        self.assertIn(c["verification"]["scope"], plan)
        self.assertIn("self.is_v12() || self.is_v13()", plan)
        verifier = (ROOT / "crates/forever-extractor/src/verify.rs").read_text()
        self.assertIn("terrain_v13::admit_metadata(plan, artifacts)", verifier)
        self.assertIn("terrain_v12::admit_metadata(plan, artifacts)", verifier)
        self.assertEqual(c["coverage"]["authentication"], "producer_reported")
        self.assertFalse(c["coverage"]["complete_global_map_coverage_claim"])

    def test_historical_unknown_version_control_stays_unknown(self):
        source = (ROOT / "crates/forever-extractor/tests/product_v11_cli.rs").read_text()
        self.assertIn('PLAN.replace("forever-product\\t11", "forever-product\\t99")', source)

    def test_existing_source_recipe_carries_new_compile_inputs(self):
        spec = importlib.util.spec_from_file_location(
            "v13_packager", ROOT / "tools/package_forever.py")
        packager = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(packager)
        selected = set(packager.selected_paths(ROOT))
        for name in (
                "docs/forever-terrain-v13.md",
                "crates/forever-extractor/src/driver/terrain_v13.rs",
                "crates/forever-extractor/src/driver/terrain_v13/tests.rs",
                "crates/forever-extractor/src/plan/v13_tests.rs",
                "crates/forever-extractor/tests/product_v13_cli.rs",
                "contracts/forever-product-contract-v13.json",
                "tests/contracts/fixtures/forever-product-v13/map0.tsv",
                "tests/contracts/fixtures/forever-product-v13/available.tsv"):
            self.assertIn(name, selected)
            self.assertTrue(packager.bundle_path_allowed("source/" + name))


if __name__ == "__main__":
    unittest.main()
