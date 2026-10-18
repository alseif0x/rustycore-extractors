# SPDX-License-Identifier: GPL-3.0-or-later
"""Build-70450 terrain contract and authored plan fixtures; no client IO."""
import json
from pathlib import Path
import unittest

ROOT = Path(__file__).resolve().parents[2]
FIXTURES = ROOT / "tests/contracts/fixtures/forever-product-v20"


class ProductV20Contract(unittest.TestCase):
    def setUp(self):
        self.contract = json.loads(
            (ROOT / "contracts/forever-product-contract-v20.json").read_bytes()
        )

    def test_exact_product_identity_and_closed_scope(self):
        c = self.contract
        self.assertEqual(
            (c["contract"], c["version"], c["publication_profile"], c["builds"]),
            ("forever-product-v20", 20, "Build70450CompatibleTerrainV1", [70450]),
        )
        self.assertEqual((c["platform"], c["backend"], c["offline"]), ("linux", "rust-local", True))
        self.assertEqual(c["commands"], ["inspect", "extract", "resume", "verify", "verify-links"])
        self.assertEqual(c["plan"]["schema_catalog"], "wowdbdefs-70450-v1")
        self.assertEqual(c["plan"]["component_mask"], 8)
        self.assertEqual(c["plan"]["components"], ["maps"])
        self.assertEqual(c["plan"]["decryption"], "disabled")
        self.assertEqual(c["plan"]["storage"], "resident")
        self.assertEqual(c["plan"]["db2_output_entries"], 0)
        self.assertFalse(c["plan"]["tile_subsets"])
        self.assertEqual(c["plan"]["navigation"], "none")
        self.assertFalse(c["coverage"]["complete_global_map_coverage_claim"])
        self.assertEqual(c["coverage"]["authentication"], "producer_reported")
        self.assertFalse(c["full_extractor_complete"])

    def test_two_canonical_plans_are_closed_and_parseable_fixtures(self):
        expected = {"map2.tsv": "map:2", "available.tsv": "all-available-maps"}
        for name, terrain in expected.items():
            raw = (FIXTURES / name).read_bytes()
            self.assertTrue(raw.endswith(b"\n"))
            self.assertNotIn(b"\r", raw)
            pairs = [line.split("\t") for line in raw.decode("ascii").splitlines()]
            self.assertEqual(len(pairs), 15)
            self.assertTrue(all(len(pair) == 2 for pair in pairs))
            fields = dict(pairs)
            self.assertEqual(len(fields), 15)
            self.assertEqual(fields["forever-product"], "20")
            self.assertEqual(fields["build"], "70450")
            self.assertEqual(fields["schema-catalog"], "wowdbdefs-70450-v1")
            self.assertEqual(fields["components"], "maps")
            self.assertEqual(fields["terrain"], terrain)
            self.assertEqual(fields["decryption"], "disabled")
            self.assertEqual(fields["navigation"], "none")
            self.assertFalse(any(line.startswith("db2\t") for line in raw.decode("ascii").splitlines()))

    def test_coverage_is_explicitly_producer_reported_and_not_complete(self):
        c = self.contract
        self.assertEqual(c["source_profile"], "linux70450-compatible-full-maps-available-map-v1")
        self.assertEqual(c["coverage"]["authentication"], "producer_reported")
        self.assertFalse(c["coverage"]["complete_global_map_coverage_claim"])
        self.assertIn("not an independent client snapshot", c["verification"]["source_authentication"])
        self.assertIn("format parity", c["verification"]["both_commands"])
        self.assertIn("70450 ADT/MAPS format parity", c["excluded"])
        self.assertFalse(c["full_extractor_complete"])

    def test_runner_and_verifier_are_bound_to_compatible_70450_route(self):
        driver = (ROOT / "crates/forever-extractor/src/driver.rs").read_text()
        self.assertIn("if plan.is_v20() {", driver)
        self.assertIn("terrain_v20::extract(plan, store, run, self.context)", driver)
        verifier = (ROOT / "crates/forever-extractor/src/verify.rs").read_text()
        self.assertIn("terrain_v20::admit_metadata(plan, artifacts)", verifier)
        self.assertIn(
            "get_or_insert_with(crate::driver::terrain_v20::VerifyFacts::new)",
            verifier,
        )
        terrain = (ROOT / "crates/forever-extractor/src/driver/terrain_v20.rs").read_text()
        self.assertIn("available::run_compatible_full_maps(", terrain)
        self.assertIn("full_ckey_verified", terrain)
        self.assertIn("encoded_key_authenticated", terrain)
        self.assertIn("available::COMPATIBLE_FULL_MAP_PROFILE", terrain)


if __name__ == "__main__":
    unittest.main()
